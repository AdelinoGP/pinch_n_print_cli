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
//! graph: ports of canonical `generateJunctions`, `connectJunctions`,
//! `addToolpathSegment` and `generateLocalMaximaSingleBeads`
//! (`SkeletalTrapezoidation.cpp`).
//!
//! - For every upward half-edge (ribs included — no type/centrality gate,
//!   matching canonical `generateJunctions`), [`generate_junctions`] resolves
//!   ONE `Beading` at the edge's peak (`to`, the higher-`distance_to_boundary`
//!   endpoint) via the `BeadingPropagation` side table
//!   ([`crate::skeletal_trapezoidation::SkeletalTrapezoidationGraph::get_beding`]/
//!   `get_nearest_beding`, falling back to `BeadingStrategy::compute()`), and
//!   emits only the in-band beads of that single beading, reading each
//!   junction's width directly from the beading's own `bead_widths[idx]`.
//! - [`connect_junctions`] seeds `unprocessed_quad_starts` from every live
//!   edge whose `.prev` is absent (every rib back edge plus each cell's first
//!   edge), walks each domain quad by quad ([`find_quad`], then the quad end's
//!   `.twin`, canonical `getNextUnconnected`), and inside each quad connects
//!   the fan on the edge(s) rising to the peak ([`quad_max_r_edge_to`],
//!   canonical `getQuadMaxRedgeTo`) with the fan on the edge(s) falling from
//!   it, one [`add_toolpath_segment`] per bead. A segment extends the inset's
//!   last line when it starts where that line ends; otherwise it starts a new
//!   line, and `stitch_extrusions` joins the pieces later.
//!
//! Every quad draws its own segments. The previous emission concatenated,
//! per bead, the junctions of every junction-bearing edge met along a whole
//! domain walk into one polyline. That drew straight chords wherever a bead
//! was absent for a stretch of quads, dropped the closing segment of every
//! ring (the first and last junctions were merged instead of joined), and
//! dropped a quad's walls whenever the walk started or stopped next to it.
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
use crate::skeletal_trapezoidation::centrality::is_local_maximum;
use crate::skeletal_trapezoidation::SkeletalTrapezoidationGraph;
use crate::voronoi::{Vertex, NO_INDEX};

/// Per-edge junction fans produced by `generate_junctions`.
///
/// One flat `Vec<ExtrusionJunction>` per (upward) edge, matching
/// OrcaSlicer's `LineJunctions` layout (`ExtrusionJunction.hpp:85-87`). The
/// junctions are pushed in peak-side-first order (highest `distance_to_boundary`
/// / highest junction radius) to boundary-side-last (lowest radius), so
/// `junctions[0]` is the peak-side junction and the last entry is the
/// boundary-side junction. Each junction's `perimeter_index` equals its bead
/// index at emission time, mirroring the canonical `junction.perimeter_index =
/// junction_idx` assignment (`SkeletalTrapezoidation.cpp:2064-2077`).
type EdgeJunctions = Vec<ExtrusionJunction>;

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

/// Builds the per-edge flat junction fan for every
/// upward half-edge of the skeletal graph.
///
/// Mirrors OrcaSlicer's `generateJunctions`
/// (`OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2013-2079`):
///
/// - Iterates ALL graph edges — canonical's own loop (`:2015`,
///   `for (edge_t& edge_ : graph.edges)`) has NO edge-type or centrality
///   check anywhere in the function; ribs (`EdgeType::EXTRA_VD`) are genuine
///   junction carriers, not excluded (finding N1 of the second-pass audit —
///   the previous implementation's retained `edge.central`/`EXTRA_VD` gates
///   here were themselves the bug, not a faithful adaptation of one).
/// - Skips the downward half of each twin pair: `from.R > to.R` continues
///   (`:2017-2019`) — the OTHER half-edge owns the emission. A flat edge
///   (both endpoints equal R) has neither half upward, so it is silently
///   dropped.
/// - Skips flat / same-bead-count edges (`:2024-2027`): if the from-end
///   already carries the SAME bead count as the to-end, there is no
///   transition crossing this edge and hence no in-band bead to emit.
/// - Resolves ONE `Beading` per upward edge at its **peak** (`to_idx`, the
///   HIGHER-R endpoint) — `getBeading(edge->to, ...)` at `:2029` — via
///   [`SkeletalTrapezoidationGraph::get_beding`], falling back to
///   [`SkeletalTrapezoidationGraph::get_nearest_beding`] and finally to
///   `strategy.compute(2 * to_r, bead_count)` when the N7 side table has no
///   entry yet for that vertex.
/// - Emits ONLY in-band beads: the scan starts at the middle bead index
///   `(max(1, n) - 1) / 2` (`:2046`) and searches toward index 0 for the
///   outermost bead whose radius is within the peak's `distance_to_boundary`
///   (`:2046-2055`), then walks outward from there; the loop breaks once a
///   bead's radius falls below the edge's lower-R end (`:2068`) — out-of-band
///   beads are skipped entirely, never clamped onto an endpoint.
/// - Each emitted junction's width is the resolved beading's OWN
///   `bead_widths[idx]` (`:2076`, `beading->bead_widths[junction_idx]`) —
///   never recomputed per bead from a fresh `strategy.compute()` call.
///
/// This entry point is `pub` (not `pub(crate)`) so the
/// `arachne_junction_upward_half_edge_only` integration test can pin the
/// AC-N1 contract — "only the upward half-edge of a twin pair emits
/// junctions" — directly on the function, without going through the
/// downstream `connectJunctions` chain walk. The function is otherwise
/// internal to this module; the public surface is [`generate_toolpaths`].
pub fn generate_junctions(
    graph: &SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) -> BTreeMap<usize, EdgeJunctions> {
    let mut edge_junctions: BTreeMap<usize, EdgeJunctions> = BTreeMap::new();

    // Radius (slicer units) `get_nearest_beding` searches when the peak
    // vertex has no populated `BeadingPropagation` side-table entry of its
    // own. Matches OrcaSlicer's `getNearestBeading` 0.1 mm default
    // (`SkeletalTrapezoidation.cpp:2098-2127`).
    const NEAREST_BEDING_RADIUS_UNITS: f64 = 0.1 * UNITS_PER_MM;

    // Near-start-R snap tolerance: a bead whose `bead_R` lands within this
    // many slicer units of the edge's `start_R` (the peak's
    // `distance_to_boundary`) snaps its junction to the peak vertex
    // (matching OrcaSlicer's `if (bead_R > start_R - epsilon)` clamp at
    // `SkeletalTrapezoidation.cpp:2064-2077`). 0.01 mm = 100 slicer units at
    // `UNITS_PER_MM = 10_000`. Expressed in the same units as
    // `distance_to_boundary` so the comparison stays in the scaled-integer
    // domain and never round-trips through `f32`.
    const NEAR_START_R_TOL_UNITS: f64 = 0.01 * UNITS_PER_MM;

    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        // No type/centrality gate here — see this function's doc comment.
        // Ribs and non-central edges walk the same selection/beading logic
        // as any other edge; the downstream domain walk in
        // `generate_toolpaths` is what threads them into one chain.
        let to_idx = resolve_to_vertex(graph, edge_idx);
        let Some(end_vertex) = graph.vertices.get(to_idx) else {
            continue;
        };
        let Some(start_vertex) = graph.vertices.get(edge.start_vertex) else {
            continue;
        };
        // Canonical `generateJunctions` gate (`SkeletalTrapezoidation.cpp:1740-1744`).
        // The DOWNWARD half-edge (start_R higher than the peak) is skipped so
        // the twin owns emission. Canonical compares RAW bead counts (default
        // -1 for "unassigned") BEFORE any `getOrCreateBeading` synthesis, and
        // skips ONLY when both endpoints already carry an equal, NON-NEGATIVE
        // bead count (no transition crosses the edge), or the edge is not
        // upward. Crucially it does NOT skip a peak whose bead count is
        // unassigned (`-1` / `None`): a non-central taper spine node (e.g. a
        // benchy bow, whose converging-side bisector legitimately fails the
        // centrality predicate and never receives a primary bead count) still
        // must emit, pulling its widths from `getOrCreateBeading(edge->to)`.
        // PnP formerly `continue`d whenever `end_vertex.bead_count` was
        // `None`/`0`, dropping every wall in such tapered regions — the D5
        // benchy-bow dropout. See `docs/DEVIATION_LOG.md`.
        let from_r = start_vertex.distance_to_boundary;
        let to_r = end_vertex.distance_to_boundary;
        let raw_from_bc: i64 = start_vertex.bead_count.map_or(-1, |n| i64::from(n));
        let raw_to_bc: i64 = end_vertex.bead_count.map_or(-1, |n| i64::from(n));
        if from_r >= to_r || (raw_from_bc == raw_to_bc && raw_from_bc >= 0) {
            continue;
        }

        // `getOrCreateBeading` synthesis (`SkeletalTrapezoidation.cpp:1808-1839`):
        // a peak vertex with no assigned bead count derives one from its own
        // thickness (`getOptimalBeadCount(distance_to_boundary * 2)`), so
        // emission never depends on a prior positive bead-count assignment.
        let bead_count: u32 = match end_vertex.bead_count {
            Some(n) => n,
            None => strategy.optimal_bead_count(2.0 * to_r) as u32,
        };
        if bead_count == 0 {
            // A genuinely sub-optimal-width peak resolves to zero beads; its
            // beading is empty so there is nothing to emit. (Canonical's
            // `getOrCreateBeading` still runs, but `compute()` yields an empty
            // beading — skipping early here is equivalent.)
            continue;
        }

        // Resolve ONE beading at the peak (`to_idx`, the higher-R vertex) —
        // matches canonical's `getBeading(edge->to, ...)`. The N7 side table
        // is checked first (exact populated entry, then a bounded nearest-
        // neighbor search); `strategy.compute()` is the last-resort fallback
        // for a graph whose side table was never populated (e.g. a caller
        // that skips `populate_beading_propagation`).
        let peak_beading = match graph.get_beding(to_idx) {
            Some(b) if !b.bead_widths.is_empty() => b.clone(),
            _ => match graph.get_nearest_beding(to_idx, NEAREST_BEDING_RADIUS_UNITS) {
                Some(b) if !b.bead_widths.is_empty() => b.clone(),
                _ => strategy.compute(2.0 * to_r, bead_count as usize),
            },
        };

        // Hot-path invariant from `Beading`'s doc: `bead_widths` and
        // `toolpath_locations` are index-parallel, ordered outermost to
        // innermost. `populate_beading_propagation` enforces this for
        // stored beadings; the `compute` fallback enforces it via
        // `assert_beading_invariant` inside the strategy stack. A
        // regression in either surfaces here, on the first `generate_`
        // call after the bug lands, not deep in a downstream consumer.
        debug_assert_eq!(
            peak_beading.bead_widths.len(),
            peak_beading.toolpath_locations.len(),
            "beading at peak vertex {to_idx} violates bead_widths.len() == toolpath_locations.len()"
        );
        let n = peak_beading.bead_widths.len();
        if n == 0 {
            continue;
        }

        // In-band bead emission. `start_r` = peak (this edge's `to_r`);
        // `end_r` = boundary-side (this edge's `from_r`) — matching
        // canonical's own `start_R`/`end_R` naming in `generateJunctions`
        // (the snap-to-`a` "start node" in the C++ source is the peak, per
        // the interpolation below).
        let start_r = to_r;
        let end_r = from_r;
        let start_r_mm = start_r / UNITS_PER_MM;
        let end_r_mm = end_r / UNITS_PER_MM;
        let from_pos = start_vertex.position;
        let to_pos = end_vertex.position;
        let (sx, sy) = to_mm_xy(from_pos);
        let (ex, ey) = to_mm_xy(to_pos);

        // First pass: find the outermost in-band bead index via the
        // canonical mid-to-outer scan (`SkeletalTrapezoidation.cpp:
        // 2046-2055`) — start at the middle bead index and walk toward 0
        // (outward) until a bead's radius is within 1 slicer unit of the
        // peak's own R.
        let num_junctions = peak_beading.toolpath_locations.len();
        let mut start_idx: Option<usize> = None;
        {
            let mut scan: isize = (num_junctions.max(1) - 1) as isize / 2;
            loop {
                if scan < 0 {
                    break;
                }
                let idx = scan as usize;
                let bead_r = peak_beading.toolpath_locations[idx];
                if bead_r <= start_r + 1.0 {
                    start_idx = Some(idx);
                    break;
                }
                if scan == 0 {
                    break;
                }
                scan -= 1;
            }
        }
        let Some(start_idx) = start_idx else {
            // No bead in the peak's beading lies within the edge's band.
            // Legitimate "no in-band beads" outcome; store an empty entry
            // so the downstream chain walk's `contains_key` check skips it.
            edge_junctions.insert(edge_idx, Vec::new());
            continue;
        };

        // Second pass: walk DOWNWARD from `start_idx` (the outermost
        // in-band bead) toward index 0 — matching canonical's own
        // direction ("the loop walks junction_idx downward and breaks once
        // bead_R < end_R", `SkeletalTrapezoidation.cpp:2064-2077`). Since
        // `toolpath_locations` is monotonically increasing with index
        // (outermost-to-innermost ordering), walking DOWN from `start_idx`
        // moves toward smaller radii; the walk breaks the first time a
        // bead's radius drops below the edge's lower-R end (out of band) —
        // the canonical algorithm drops such beads entirely rather than
        // clamping them onto an endpoint (`:2068`). Collected in
        // descending-index order, then reversed so the stored vector is
        // outermost-first (ascending), matching every other consumer's
        // "index 0 = outermost" convention.
        //
        // Zero-length edge guard (PNP-specific, not in canonical): when
        // `from_pos == to_pos` (e.g. a corner-rib edge where the peak
        // vertex and the R=0 boundary vertex happen to land at the same
        // spatial point — a graph-construction artifact of packet 113c's
        // rib insertion that canonical's `makeRib` / `insertRib` never
        // produces, since those project the rib endpoint to the foot of
        // the perpendicular on the source segment), the canonical formula
        // `junction = a + (b - a) * t = a` collapses every emitted
        // junction to the peak vertex's position — geometrically the
        // point ON the (zero-length) edge, but R=0.2mm is NOT that
        // point's distance_to_boundary. Skipping such an edge here (the
        // `connectJunctions` chain walk stitches the corner from the
        // surrounding non-degenerate rib segments) restores the AC-1/AC-2
        // <=0.6 / <=0.15 mm bound without weakening assertions; the beading
        // is still resolved (so `get_beding` and `is_odd` probes are
        // unaffected) and no `perimeter_index` value is lost (it was
        // going to be at the peak anyway).
        if (ex - sx).abs() <= f32::EPSILON && (ey - sy).abs() <= f32::EPSILON {
            edge_junctions.insert(edge_idx, Vec::new());
            continue;
        }
        let mut collected: Vec<ExtrusionJunction> = Vec::new();
        for idx in (0..=start_idx).rev() {
            let bead_r_units = peak_beading.toolpath_locations[idx];
            let bead_r_mm = bead_r_units / UNITS_PER_MM;

            if bead_r_mm < end_r_mm {
                break;
            }

            // Width comes directly from the resolved peak beading's own
            // array — no per-bead recompute (canonical
            // `beading->bead_widths[junction_idx]`, `:2076`).
            let width_units = peak_beading.bead_widths[idx];
            let width_mm = (width_units / UNITS_PER_MM) as f32;

            // Near-`start_R` snap (`SkeletalTrapezoidation.cpp:2072-2074`):
            // a bead whose radius is within `NEAR_START_R_TOL_UNITS` of the
            // peak's `distance_to_boundary` snaps its junction onto the
            // peak vertex so a multi-way intersection downstream sees a
            // single coincident point instead of a near-coincident pair.
            let near_start = (bead_r_units - start_r).abs() <= NEAR_START_R_TOL_UNITS;
            let t = (bead_r_mm - start_r_mm) / (end_r_mm - start_r_mm);
            let (jx, jy) = if near_start {
                (ex as f64, ey as f64)
            } else {
                // Mirrors OrcaSlicer's `junction = a + (ab * (bead_R -
                // start_R) / (end_R - start_R))` at
                // `SkeletalTrapezoidation.cpp:2071`, where `a = edge.to.p`
                // (the PEAK) and `ab = edge.from.p - edge.to.p`
                // (peak-to-boundary vector). The junction slides from the
                // peak (`t = 0`) toward the boundary-side vertex (`t = 1`)
                // as `bead_R` decreases.
                (
                    ex as f64 + (sx as f64 - ex as f64) * t,
                    ey as f64 + (sy as f64 - ey as f64) * t,
                )
            };

            // `perimeter_index` is the bead/inset index (`junction_idx`) at
            // generation time (canonical `generateJunctions`:
            // `SkeletalTrapezoidation.cpp:2064-2077`, `junction.perimeter_index
            // = junction_idx`); the pop-back merge in `connectJunctions`
            // (`:2302-2314`) keys on it. This is packet 142's N2 fix —
            // replaces the pre-fix `perimeter_index: 0` placeholder that was
            // later overwritten by the pipeline's `assign_perimeter_indices`
            // post-pass (deleted by 142 alongside this change). Setting it to
            // the bead index (not the post-reverse slot position) is
            // critical: when the in-band break fires before `idx = 0`
            // (a PNP-specific case the canonical algorithm also handles —
            // see `SkeletalTrapezoidation.cpp:2064-2077`, the loop walks
            // `junction_idx` downward and breaks once `bead_R < end_R`, so
            // the surviving junctions' `junction_idx` values are NOT
            // contiguous from 0), the post-reverse slot position no longer
            // equals the bead index, and routing the junction to the wrong
            // inset line corrupts AC-1/AC-2 (the 1.7mm / 5.0mm findings).
            collected.push(ExtrusionJunction {
                p: Point3WithWidth {
                    x: jx as f32,
                    y: jy as f32,
                    z: 0.0,
                    width: width_mm,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                perimeter_index: idx as u32,
            });
        }
        collected.reverse();
        // After the reverse the fan is outermost-first (ascending
        // `perimeter_index`, boundary side first) — the REVERSE of canonical
        // `LineJunctions`, which lists the peak-side junction first.
        // `connect_junctions` reads fans through `canonical_fan`, which
        // restores canonical order.
        edge_junctions.insert(edge_idx, collected);
    }

    edge_junctions
}

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

/// Junction fan of `edge_idx` in canonical `LineJunctions` order: the
/// highest-`perimeter_index` (peak-side) junction first, the boundary-side
/// junction last. [`generate_junctions`] stores fans outermost-first, so this
/// is the stored fan reversed. An edge with no stored fan yields an empty
/// fan, like canonical's lazily created empty `LineJunctions`.
fn canonical_fan(
    edge_junctions: &BTreeMap<usize, EdgeJunctions>,
    edge_idx: usize,
) -> Vec<ExtrusionJunction> {
    let mut fan = edge_junctions.get(&edge_idx).cloned().unwrap_or_default();
    fan.reverse();
    fan
}

/// Position within `quad` of the edge `connectJunctions` treats as the edge
/// to the quad's peak: canonical `getQuadMaxRedgeTo`
/// (`SkeletalTrapezoidation.cpp`). The first edge whose `to` vertex has the
/// strictly largest `distance_to_boundary` wins; when that is the quad's last
/// edge and it barely rises (`to.R - 0.005 mm < from.R`), its predecessor is
/// used instead so an edge from the peak always exists.
fn quad_max_r_edge_to(graph: &SkeletalTrapezoidationGraph, quad: &[usize]) -> usize {
    let r_of = |v: usize| {
        graph
            .vertices
            .get(v)
            .map(|v| v.distance_to_boundary)
            .unwrap_or(0.0)
    };
    let mut best_pos = 0;
    let mut best_r = f64::NEG_INFINITY;
    for (pos, &edge_idx) in quad.iter().enumerate() {
        let r = r_of(resolve_to_vertex(graph, edge_idx));
        if r > best_r {
            best_r = r;
            best_pos = pos;
        }
    }
    if best_pos + 1 == quad.len() && best_pos > 0 {
        let edge_idx = quad[best_pos];
        let from_r = r_of(graph.edges[edge_idx].start_vertex);
        if best_r - 0.005 * UNITS_PER_MM < from_r {
            best_pos -= 1;
        }
    }
    best_pos
}

/// Canonical `STHalfEdgeNode::isMultiIntersection`: more than two central
/// edges leave the node.
fn is_multi_intersection(graph: &SkeletalTrapezoidationGraph, v: usize) -> bool {
    graph
        .edges
        .iter()
        .filter(|e| e.start_vertex == v && e.central)
        .count()
        > 2
}

/// The per-node half of canonical `connectJunctions`' `from_is_odd` /
/// `to_is_odd`: an odd bead count and no transition at `v`, and `p` within
/// 0.005 mm of `v`.
fn is_odd_at_node(graph: &SkeletalTrapezoidationGraph, v: usize, p: &Point3WithWidth) -> bool {
    let Some(node) = graph.vertices.get(v) else {
        return false;
    };
    let Some(bead_count) = node.bead_count else {
        return false;
    };
    if bead_count == 0 || bead_count % 2 == 0 || node.transition_ratio != 0.0 {
        return false;
    }
    let (nx, ny) = to_mm_xy(node.position);
    let dx = (p.x - nx) as f64;
    let dy = (p.y - ny) as f64;
    const TOL_MM: f64 = 0.005;
    dx * dx + dy * dy <= TOL_MM * TOL_MM
}

/// Canonical `SkeletalTrapezoidation::addToolpathSegment`: appends `to` to
/// the inset's last line when that line ends at `from` (within 10 µm, same
/// width within 10 µm, not a forced break), prepends-by-appending `from` when
/// it ends at `to`, and otherwise starts a new two-junction line.
fn add_toolpath_segment(
    toolpaths: &mut BTreeMap<u32, Vec<ExtrusionLine>>,
    from: &ExtrusionJunction,
    to: &ExtrusionJunction,
    is_odd: bool,
    mut force_new_path: bool,
    from_is_3way: bool,
    to_is_3way: bool,
) {
    if from == to {
        return;
    }
    const SNAP_MM: f32 = 0.010;
    let inset_idx = from.perimeter_index;
    let lines = toolpaths.entry(inset_idx).or_default();
    match lines.last() {
        None => force_new_path = true,
        Some(last) => {
            let back_idx = last.junctions.last().map(|j| j.perimeter_index);
            if last.is_odd != is_odd || back_idx != Some(inset_idx) {
                force_new_path = true;
            }
        }
    }
    if !force_new_path {
        let last = lines.last_mut().expect("checked non-empty above");
        let back = last
            .junctions
            .last()
            .expect("lines are never created empty")
            .p;
        let near = |q: &Point3WithWidth| {
            let dx = back.x - q.x;
            let dy = back.y - q.y;
            dx * dx + dy * dy <= SNAP_MM * SNAP_MM && (back.width - q.width).abs() < SNAP_MM
        };
        if near(&from.p) && !from_is_3way {
            last.junctions.push(to.clone());
            return;
        }
        if near(&to.p) && !to_is_3way {
            last.junctions.push(from.clone());
            return;
        }
    }
    lines.push(ExtrusionLine {
        junctions: vec![from.clone(), to.clone()],
        inset_idx,
        is_odd,
        is_closed: false,
    });
}

/// Canonical `SkeletalTrapezoidation::connectJunctions`
/// (`SkeletalTrapezoidation.cpp`): walks every polygon domain quad by quad and,
/// inside each quad, connects the junction fan on the edge(s) rising to the
/// quad's peak with the fan on the edge(s) falling from it, one
/// [`add_toolpath_segment`] per bead. Every quad draws its own segments, so
/// where a domain walk starts or stops never decides whether a quad's walls
/// exist.
fn connect_junctions(
    graph: &SkeletalTrapezoidationGraph,
    edge_junctions: &BTreeMap<usize, EdgeJunctions>,
) -> BTreeMap<u32, Vec<ExtrusionLine>> {
    let mut toolpaths: BTreeMap<u32, Vec<ExtrusionLine>> = BTreeMap::new();

    // Canonical seeds every edge without a `prev`. `collapse_small_edges`
    // marks the edges canonical erases with `start_vertex == NO_INDEX` but
    // leaves them in `graph.edges` (with `prev == NO_INDEX` and a stale
    // `twin`), so they are excluded here: seeding one started a walk that
    // hopped through its stale twin into a live domain.
    let mut unprocessed_quad_starts: BTreeSet<usize> = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| edge.prev == NO_INDEX && edge.start_vertex != NO_INDEX)
        .map(|(idx, _)| idx)
        .collect();
    let mut passed_odd_edges: BTreeSet<usize> = BTreeSet::new();

    while let Some(&poly_domain_start) = unprocessed_quad_starts.iter().next() {
        let mut quad_start = poly_domain_start;
        let mut new_domain_start = true;
        loop {
            unprocessed_quad_starts.remove(&quad_start);
            let quad = find_quad(graph, quad_start);
            let quad_end = *quad.last().expect("find_quad returns at least one edge");
            let peak_pos = quad_max_r_edge_to(graph, &quad);

            // Canonical asserts an edge from the peak exists; a one-edge
            // chain has none and connects nothing.
            if peak_pos + 1 < quad.len() {
                let edge_to_peak = quad[peak_pos];
                let edge_from_peak = quad[peak_pos + 1];
                let twin_of = |e: usize| graph.edges.get(e).map_or(NO_INDEX, |edge| edge.twin);

                // Junctions from the quad start up to the peak.
                let mut from_junctions = canonical_fan(edge_junctions, edge_to_peak);
                if peak_pos > 0 {
                    let from_prev = canonical_fan(edge_junctions, quad[peak_pos - 1]);
                    while let (Some(back), Some(front)) = (from_junctions.last(), from_prev.first()) {
                        if back.perimeter_index > front.perimeter_index {
                            break;
                        }
                        from_junctions.pop();
                    }
                    from_junctions.extend(from_prev);
                }
                // Junctions from the quad end up to the peak.
                let mut to_junctions = canonical_fan(edge_junctions, twin_of(edge_from_peak));
                if peak_pos + 2 < quad.len() {
                    let to_next = canonical_fan(edge_junctions, twin_of(quad[peak_pos + 2]));
                    while let (Some(back), Some(front)) = (to_junctions.last(), to_next.first()) {
                        if back.perimeter_index > front.perimeter_index {
                            break;
                        }
                        to_junctions.pop();
                    }
                    to_junctions.extend(to_next);
                }

                let quad_start_to = resolve_to_vertex(graph, quad_start);
                let quad_end_from = graph.edges[quad_end].start_vertex;
                let quad_start_next = graph.edges[quad_start].next;
                let segment_count = from_junctions.len().min(to_junctions.len());
                for junction_rev_idx in 0..segment_count {
                    let from = &from_junctions[from_junctions.len() - 1 - junction_rev_idx];
                    let to = &to_junctions[to_junctions.len() - 1 - junction_rev_idx];
                    let innermost = junction_rev_idx + 1 == segment_count;
                    let from_is_odd = innermost && is_odd_at_node(graph, quad_start_to, &from.p);
                    let to_is_odd = innermost && is_odd_at_node(graph, quad_end_from, &to.p);
                    let is_odd_segment = from_is_odd && to_is_odd;
                    // Only generate toolpath for odd segments once.
                    if is_odd_segment
                        && quad_start_next != NO_INDEX
                        && passed_odd_edges.contains(&twin_of(quad_start_next))
                    {
                        continue;
                    }
                    let from_is_3way = from_is_odd && is_multi_intersection(graph, quad_start_to);
                    let to_is_3way = to_is_odd && is_multi_intersection(graph, quad_end_from);
                    if quad_start_next != NO_INDEX {
                        passed_odd_edges.insert(quad_start_next);
                    }
                    add_toolpath_segment(
                        &mut toolpaths,
                        from,
                        to,
                        is_odd_segment,
                        new_domain_start,
                        from_is_3way,
                        to_is_3way,
                    );
                }
            }
            new_domain_start = false;

            // `getNextUnconnected`: the twin of the quad's last edge starts
            // the next quad. Canonical loops until it is back at the domain
            // start; a missing or already-walked twin (malformed topology)
            // ends the walk instead of looping.
            let next_start = graph.edges.get(quad_end).map_or(NO_INDEX, |e| e.twin);
            if next_start == poly_domain_start || !unprocessed_quad_starts.contains(&next_start) {
                break;
            }
            quad_start = next_start;
        }
    }
    toolpaths
}

/// Emits 6-segment hexagonal micro-loops at local maxima with odd bead count,
/// mirroring canonical `generateLocalMaximaSingleBeads`
/// (`SkeletalTrapezoidation.cpp`).
///
/// For each vertex whose beading has an odd `bead_widths` count,
/// `is_local_maximum` (strict — matching canonical `isLocalMaximum(true)`),
/// and no central incident edge, emits a closed `ExtrusionLine` hexagon
/// (radius `width/8` where `width` is the middle bead's width, `is_odd = true`)
/// so isolated thick spots get their center dot.
fn generate_local_maxima_single_beads(
    graph: &SkeletalTrapezoidationGraph,
    buckets: &mut BTreeMap<u32, Vec<ExtrusionLine>>,
) {
    use std::f64::consts::TAU;

    for (v_idx, vertex) in graph.vertices.iter().enumerate() {
        // Gate 1: odd bead count from the beading side table.
        let Some(beading) = graph.get_beding(v_idx) else {
            continue;
        };
        let n_beads = beading.bead_widths.len();
        if n_beads == 0 || n_beads % 2 == 0 {
            continue;
        }

        // Gate 2: strict local maximum
        if !is_local_maximum(graph, v_idx) {
            continue;
        }

        // Gate 3: not central — no edge starting from this vertex is central.
        if graph
            .edges
            .iter()
            .any(|e| e.start_vertex == v_idx && e.central)
        {
            continue;
        }

        // Emit a 6-segment hexagonal micro-loop.
        let mid_bead = n_beads / 2;
        let width = beading.bead_widths[mid_bead];
        let r = width / 8.0; // radius in slicer units

        let cx = vertex.position.x / UNITS_PER_MM;
        let cy = vertex.position.y / UNITS_PER_MM;
        let r_mm = (r / UNITS_PER_MM) as f32;
        let width_mm = (width / UNITS_PER_MM) as f32;

        // 6 hexagon vertices plus a 7th duplicating the first. A closed
        // `ExtrusionLine` in this crate carries `first.xy == last.xy` (the
        // convention `stitch_extrusions` produces and `ExtrusionPath3D::is_closed`
        // documents), and `simplify_toolpaths`' closed-polygon walk relies on it.
        // This loop skips stitch (AC-6), so it must close itself.
        let mut junctions = Vec::with_capacity(7);
        for seg in 0..6usize {
            let angle = TAU * seg as f64 / 6.0;
            let jx = cx as f32 + r_mm * angle.cos() as f32;
            let jy = cy as f32 + r_mm * angle.sin() as f32;
            junctions.push(ExtrusionJunction {
                p: Point3WithWidth {
                    x: jx,
                    y: jy,
                    z: 0.0,
                    width: width_mm,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                perimeter_index: mid_bead as u32,
            });
        }
        // Close the loop: duplicate the start junction onto the end.
        junctions.push(junctions[0].clone());

        buckets
            .entry(mid_bead as u32)
            .or_default()
            .push(ExtrusionLine {
                junctions,
                inset_idx: mid_bead as u32,
                is_odd: true,
                is_closed: true,
            });
    }
}

/// Emits variable-width toolpath insets from `graph`, sourcing every bead's
/// width and toolpath offset from `strategy`: [`generate_junctions`], then
/// [`connect_junctions`], then the local-maxima single beads (canonical
/// `generateSegments`' tail). Every emitted line is open (`is_closed =
/// false`); [`super::stitch::stitch_extrusions`] joins and closes them.
///
/// Returns one [`VariableWidthLines`] bucket per distinct `inset_idx`, sorted
/// ascending (`0` = outermost).
pub fn generate_toolpaths(
    graph: &SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) -> Vec<VariableWidthLines> {
    let edge_junctions = generate_junctions(graph, strategy);
    let mut buckets = connect_junctions(graph, &edge_junctions);

    // N9: emit hexagonal micro-loops at isolated local-maxima thick spots
    // with odd bead count (OrcaSlicer `generateLocalMaximaSingleBeads`).
    generate_local_maxima_single_beads(graph, &mut buckets);

    buckets.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn junction(x: f32, y: f32, width: f32, perimeter_index: u32) -> ExtrusionJunction {
        ExtrusionJunction {
            p: Point3WithWidth {
                x,
                y,
                width,
                ..Point3WithWidth::default()
            },
            perimeter_index,
        }
    }

    fn xy(line: &ExtrusionLine) -> Vec<(f32, f32)> {
        line.junctions.iter().map(|j| (j.p.x, j.p.y)).collect()
    }

    /// Canonical `addToolpathSegment` continuation rules: a segment starting
    /// where the inset's last line ends extends it; one ending there is
    /// appended by its `from` end; anything else — or a forced new path, or
    /// a parity change — starts a new two-junction line; a zero-length
    /// segment is dropped.
    #[test]
    fn add_toolpath_segment_extends_only_a_line_ending_at_the_segment() {
        let mut toolpaths: BTreeMap<u32, Vec<ExtrusionLine>> = BTreeMap::new();
        let a = junction(0.0, 0.0, 0.4, 1);
        let b = junction(1.0, 0.0, 0.4, 1);
        let c = junction(2.0, 0.0, 0.4, 1);
        let d = junction(2.0, 1.0, 0.4, 1);
        let far = junction(5.0, 5.0, 0.4, 1);

        add_toolpath_segment(&mut toolpaths, &a, &b, false, true, false, false);
        add_toolpath_segment(&mut toolpaths, &b, &c, false, false, false, false);
        // Reversed segment ending at the line's last point: `from` is appended.
        add_toolpath_segment(&mut toolpaths, &d, &c, false, false, false, false);
        let lines = &toolpaths[&1];
        assert_eq!(lines.len(), 1);
        assert_eq!(xy(&lines[0]), vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (2.0, 1.0)]);

        // Not touching the last point: a new line, never a chord to it.
        add_toolpath_segment(&mut toolpaths, &far, &a, false, false, false, false);
        // Touching, but a new domain forces a new path.
        add_toolpath_segment(&mut toolpaths, &a, &b, false, true, false, false);
        // Touching, but the parity differs.
        add_toolpath_segment(&mut toolpaths, &b, &c, true, false, false, false);
        // Zero-length: dropped.
        add_toolpath_segment(&mut toolpaths, &c, &c, true, false, false, false);
        let lines = &toolpaths[&1];
        assert_eq!(lines.len(), 4);
        assert_eq!(xy(&lines[1]), vec![(5.0, 5.0), (0.0, 0.0)]);
        assert_eq!(xy(&lines[2]), vec![(0.0, 0.0), (1.0, 0.0)]);
        assert_eq!(xy(&lines[3]), vec![(1.0, 0.0), (2.0, 0.0)]);
        assert!(lines[3].is_odd && !lines[2].is_odd);
        assert!(lines.iter().all(|l| !l.is_closed && l.inset_idx == 1));
    }
}
