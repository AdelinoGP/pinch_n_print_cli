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
//! graph (T-223, packet 112 Step 4 of the M2 Arachne port; reworked in Step
//! 9D to source widths/offsets from `BeadingStrategy::compute()` instead of a
//! geometric approximation).
//!
//! # Honesty note (no OrcaSlicer oracle; ADAPTATION, not a literal port)
//!
//! Per this packet's brief, `generateToolpaths()` upstream is only a
//! phase-2 *orchestrator* (`updateIsCentral → filterCentral →
//! filterOuterCentral → updateBeadCount → filterNoncentralRegions →
//! generateTransitioningRibs → generateExtraRibs → generateSegments`); the
//! actual inset-emission logic lives in `generateSegments()`'s callees
//! `generateJunctions()` (which walks each upward half-edge and, for every
//! bead radius in `[end_R, start_R)`, interpolates a 2D point *along the
//! edge* and emits a junction, with width/location sourced from that node's
//! precomputed `Beading`) and `connectJunctions()` (which stitches those
//! per-edge junction fans across a quad's rib/non-rib edge chain into full
//! `ExtrusionLine`s).
//!
//! That upstream machinery depends on a "quad" decomposition (ribs from the
//! polygon outline to the local-R-maximum edge) this crate's graph does not
//! have — see [`crate::skeletal_trapezoidation::graph`]'s own design notes:
//! our graph is a direct 1:1 wrap of the raw `boostvoronoi` segment diagram,
//! with no quad/rib topology. What follows is therefore a
//! **from-first-principles adaptation**, true to the *name and intent* of
//! `generateSegments`/`generateJunctions`, not a literal port:
//!
//! - **Per-junction width and offset, from the `BeadingStrategy`.** Each
//!   central edge with `bead_count = Some(n)` (`n > 0`) has two endpoints,
//!   each with its own `distance_to_boundary` (`r_start`/`r_end`). This pass
//!   calls `strategy.compute(2.0 * r_start, n)` and
//!   `strategy.compute(2.0 * r_end, n)` — one `Beading` per endpoint — and
//!   reads bead `i`'s width and toolpath offset directly from
//!   `beading.bead_widths[i]` / `beading.toolpath_locations[i]` (both in
//!   slicer units, converted to mm via `UNITS_PER_MM`). This is what makes
//!   the strategy stack's actual behavior (e.g. `WideningBeadingStrategy`'s
//!   `min_output_width` clamp, `DistributedBeadingStrategy`'s width
//!   distribution) observable in the emitted toolpaths at all — the
//!   pre-Step-9D implementation derived width purely as
//!   `2 * distance_to_boundary / bead_count` and never called
//!   `BeadingStrategy::compute()`, so none of the composed stack's
//!   width-shaping logic ever reached the output.
//! - **Per-edge, per-bead segments — no cross-edge stitching here.** For
//!   each of an edge's `n` beads (`bead = 0..n`, `bead` doubling as
//!   `inset_idx`/`perimeter_index`), this pass emits one 2-junction
//!   `ExtrusionLine` spanning just that edge's own two endpoints. Joining
//!   adjacent edges' fragments into full inset rings is deliberately left to
//!   [`super::stitch::stitch_extrusions`] (T-225, already implemented
//!   elsewhere in this packet) rather than re-implemented here — this
//!   function's job (per AC-4's graph-only signature, now `graph` plus a
//!   `&dyn BeadingStrategy`) is per-edge emission, not topology walking.
//! - **Bead placement ("+ toolpath offset").** With no quad/rib topology,
//!   there is no way to recover *which side* of a central edge is physically
//!   nearer the real object boundary purely from the graph (no boundary
//!   polygon is passed in). Each bead is offset from its endpoint's raw
//!   graph position along a synthetic direction perpendicular to the edge,
//!   by exactly `beading.toolpath_locations[i]` (units → mm) — the
//!   strategy's own outer-edge-relative centerline offset for that bead,
//!   rather than a synthetic per-step multiple of a nominal width. This does
//!   **not** claim to reproduce which physical side of the material a given
//!   inset sits on — only that the *magnitude* of each bead's offset from
//!   the region's outer edge now comes from the strategy that is supposed to
//!   own that decision, matching the *ordering* contract (0 = outermost,
//!   increasing inward) without claiming geometric parity with a true radial
//!   reconstruction.
//! - **Guard against a short or degenerate `Beading`.** `Beading`'s own
//!   contract (`bead_widths.len() == toolpath_locations.len()`) is only
//!   debug-asserted by each strategy's `compute()`, not enforced by the type
//!   itself, and nothing here assumes a strategy always returns exactly `n`
//!   beads for a `compute(_, n)` call. In particular, this crate assigns one
//!   `bead_count` per **edge** from an `r_avg` of both endpoints (see
//!   [`crate::skeletal_trapezoidation::bead_count`]'s own honesty note on why
//!   there is no per-node bead count as upstream has), so it is routine for
//!   one endpoint of a central edge to sit essentially on the object boundary
//!   (`distance_to_boundary ≈ 0`, e.g. a polygon corner) while the other sits
//!   deep in the interior — `compute(0.0, n)` at the boundary endpoint is a
//!   well-formed call the strategy correctly answers with zero beads (there
//!   is no real thickness to print there), while the interior endpoint's own
//!   call legitimately returns all `n`. Emitting nothing for the whole edge
//!   whenever *either* endpoint under-delivers would silently drop every
//!   central edge that happens to touch a sharp corner (observably: it would
//!   zero out this crate's own square-fixture regression test, since every
//!   one of a square's central spokes runs corner-to-center). This pass
//!   therefore computes the number of beads to emit for an edge as the
//!   minimum of the requested `bead_count` and the **larger** of the two
//!   endpoints' own usable (`bead_widths`/`toolpath_locations`-length-agreed)
//!   lengths, so a genuinely printable endpoint is never suppressed by its
//!   degenerate sibling. Per bead index, each junction still prefers its
//!   *own* endpoint's strategy output; only when that endpoint's Beading ran
//!   out for this index does it fall back to the *other* endpoint's own
//!   real strategy-computed value (never a fabricated/synthetic width) —
//!   this can only happen within the index range the shorter endpoint
//!   couldn't reach, which is provably still in range on the longer
//!   endpoint's side. Never indexes past a vector's actual length, and never
//!   panics if a strategy under-delivers on one or both sides.
//! - **Canonical direction, not both twins.** A central edge and its twin
//!   always carry identical `r_min`/`r_max`/`bead_count` (computed
//!   order-independently from the same two endpoints — see
//!   [`crate::skeletal_trapezoidation::graph`]'s `edge_radius_bounds`), so
//!   walking both directions would emit exact mirrored duplicates. This pass
//!   emits from the lower-indexed half-edge of each twin pair only
//!   (`edge_idx <= twin`, or unconditionally when `twin ==
//!   `[`crate::voronoi::NO_INDEX`]`).
//! - **Z coordinate.** This graph is a per-layer 2D construct (see
//!   [`crate::skeletal_trapezoidation::graph::STVertex::position`]); every
//!   emitted junction gets `z = 0.0`. Callers that need a real layer Z must
//!   translate the result themselves (mirroring
//!   [`slicer_ir::variable_width`]'s own `z = 0.0` placeholder convention).
//! - **`is_odd`/`is_closed`.** `is_odd = (inset_idx % 2 == 1)`, matching this
//!   packet's simplified bead-parity convention used elsewhere in the
//!   Arachne post-process stack (`super::remove_small`,
//!   `super::stitch`). `is_closed` is computed directly from the emitted
//!   2-junction line's own endpoints (XY-only, matching
//!   `ExtrusionPath3D::is_closed()`'s convention) — true only for the rare
//!   degenerate case of a near-zero-length source edge, since a normal
//!   2-point line spanning two distinct graph vertices is never closed by
//!   itself; real ring closure is `stitch_extrusions`'s job, matching
//!   upstream's own division of labor (`addToolpathSegment` never touches
//!   `is_closed`; only `stitchToolPaths` does).
//!
//! `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` is
//! a **self-captured regression baseline**: it locks in *this*
//! implementation's own output for regression purposes only. It is not, and
//! must never be described as, independently-derived OrcaSlicer ground
//! truth. It was re-recorded in Step 9D since the width/offset source
//! changed from a geometric approximation to `BeadingStrategy::compute()`.
//!
//! Deterministic (edges walked in ascending index order; buckets keyed by a
//! `BTreeMap<u32, _>` on the integer `inset_idx`, never a float or a
//! `HashMap`; `strategy.compute()` is required to be a pure function of its
//! arguments — see [`crate::beading::BeadingStrategy`]'s own contract) and
//! panic-free (every fallible lookup degrades to "skip this edge" via
//! `let-else`, and the bead loop is clamped to the larger of the two
//! endpoints' usable `Beading` lengths, with per-index fallback to the other
//! endpoint when one side ran out, rather than ever indexing out of bounds).

use std::collections::BTreeMap;

use slicer_ir::{
    ExtrusionJunction, ExtrusionLine, Point3WithWidth, VariableWidthLines, UNITS_PER_MM,
};

use crate::beading::BeadingStrategy;
use crate::skeletal_trapezoidation::SkeletalTrapezoidationGraph;
use crate::voronoi::{Vertex, NO_INDEX};

/// XY distance below which a generated 2-junction line's own endpoints are
/// considered coincident (`is_closed = true`). Millimeters, matching
/// `Point3WithWidth`'s coordinate unit. Only ever relevant for a
/// near-zero-length source edge (see this module's doc comment).
const CLOSURE_EPS_MM: f32 = 1e-4;

/// Resolves a half-edge's "to" vertex index via its twin's `start_vertex`,
/// matching [`crate::skeletal_trapezoidation::graph`]'s own convention (see
/// that module's doc comment, and the identically-named private helpers in
/// `centrality.rs`/`propagation.rs` — duplicated here rather than shared,
/// matching this packet's existing per-module convention of small
/// self-contained helpers). Returns [`NO_INDEX`] if unresolvable
/// (missing/out-of-range twin).
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
/// millimeters, matching `Point3WithWidth`'s coordinate unit.
fn to_mm_xy(v: Vertex) -> (f32, f32) {
    ((v.x / UNITS_PER_MM) as f32, (v.y / UNITS_PER_MM) as f32)
}

/// Unit vector perpendicular to direction `(dx, dy)` (rotated 90° CCW),
/// falling back to a fixed `(0.0, 1.0)` direction for a near-zero-length
/// input rather than dividing by (near-)zero. Used to place each bead's
/// toolpath offset from its endpoint's raw graph position (see this
/// module's doc comment's "Bead placement" section) — not a claim of any
/// particular physical meaning.
fn perpendicular_unit(dx: f32, dy: f32) -> (f32, f32) {
    let len = (dx * dx + dy * dy).sqrt();
    if len > f32::EPSILON {
        (-dy / len, dx / len)
    } else {
        (0.0, 1.0)
    }
}

/// Squared XY distance between two junction positions (matches
/// `ExtrusionPath3D::is_closed()`'s XY-only convention).
fn dist_sq_xy(a: Point3WithWidth, b: Point3WithWidth) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// Bounds a [`crate::beading::Beading`] to the number of indices where its
/// `bead_widths`/`toolpath_locations` vectors agree — defensive, since the
/// `Beading` type only debug-asserts that invariant (see this module's doc
/// comment's "Guard against a short or degenerate `Beading`" section).
fn usable_len(b: &crate::beading::Beading) -> usize {
    b.bead_widths.len().min(b.toolpath_locations.len())
}

/// Emits variable-width toolpath insets from `graph`'s central, bead-counted
/// edges, sourcing every bead's width and toolpath offset from `strategy`.
///
/// Returns one [`VariableWidthLines`] bucket per distinct `inset_idx`
/// observed among the graph's central edges, with the outer `Vec` sorted by
/// `inset_idx` ascending (`0` = outermost). Every [`ExtrusionLine`] emitted
/// into a given bucket carries that bucket's own `inset_idx`. See this
/// module's doc comment for the full adaptation contract (strategy-sourced
/// width/offset derivation, per-edge emission, canonical-direction dedup,
/// `z = 0.0`, `is_odd`/`is_closed`).
///
/// For each central edge with `bead_count = Some(n)` (`n > 0`), this calls
/// `strategy.compute(2.0 * distance_to_boundary, n)` once per endpoint (both
/// in slicer units) and reads bead `i`'s width/offset from the resulting
/// `Beading`, converting units → mm via [`slicer_ir::UNITS_PER_MM`]. The
/// number of beads actually emitted for an edge is clamped to `n` and the
/// LARGER of the two endpoints' usable `bead_widths`/`toolpath_locations`
/// lengths (see this module's doc comment's "Guard against a short or
/// degenerate `Beading`" section for why `min` would wrongly zero out every
/// edge that touches a sharp corner) — a strategy that returns fewer beads
/// than requested on one side falls back to the other side's own real
/// output for that index, and is never indexed out of bounds.
///
/// Deterministic and panic-free.
pub fn generate_toolpaths(
    graph: &SkeletalTrapezoidationGraph,
    strategy: &dyn BeadingStrategy,
) -> Vec<VariableWidthLines> {
    let mut buckets: BTreeMap<u32, Vec<ExtrusionLine>> = BTreeMap::new();

    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        if !edge.central {
            continue;
        }
        let Some(bead_count) = edge.bead_count else {
            continue;
        };
        if bead_count == 0 {
            continue;
        }
        if edge.twin != NO_INDEX && edge_idx > edge.twin {
            // Canonical direction only: a central edge and its twin always
            // carry identical r_min/r_max/bead_count (see this module's doc
            // comment), so emit from the lower-indexed half-edge only to
            // avoid mirrored duplicates.
            continue;
        }

        let Some(start_vertex) = graph.vertices.get(edge.start_vertex) else {
            continue;
        };
        let to_idx = resolve_to_vertex(graph, edge_idx);
        let Some(end_vertex) = graph.vertices.get(to_idx) else {
            continue;
        };

        // Strategy-sourced widths/offsets (see this module's doc comment for
        // why this replaced an earlier per-junction local-R geometric
        // approximation): each endpoint gets its own Beading, computed from
        // that endpoint's own distance_to_boundary.
        let beading_start =
            strategy.compute(2.0 * start_vertex.distance_to_boundary, bead_count as usize);
        let beading_end =
            strategy.compute(2.0 * end_vertex.distance_to_boundary, bead_count as usize);

        // Guard: never index past a strategy's actually-returned bead count
        // (see this module's doc comment's "Guard against a short Beading /
        // degenerate endpoint" section). A single central edge in this
        // crate's graph can legitimately run from a vertex sitting exactly on
        // the object boundary (distance_to_boundary == 0, e.g. a polygon
        // corner) to a vertex deep in the interior — this crate assigns one
        // bead_count per EDGE from an r_avg of both endpoints (see
        // `crate::skeletal_trapezoidation::bead_count`'s own honesty note),
        // so `compute(0.0, n)` at the boundary endpoint is a well-formed call
        // that the strategy correctly answers with zero beads (there is no
        // real thickness to print there), while the interior endpoint's own
        // call legitimately returns all `n`. `usable_len` bounds a Beading to
        // indices where both its vectors agree (defensive: only
        // debug-asserted by each strategy, not enforced by the type).
        let start_len = usable_len(&beading_start);
        let end_len = usable_len(&beading_end);
        // Emit up to as many beads as the BETTER-informed endpoint supports
        // (never more than the requested bead_count): a degenerate endpoint
        // at one end of the edge must not suppress a genuinely printable
        // region at the other end.
        let effective_count = (bead_count as usize).min(start_len.max(end_len));

        let (sx, sy) = to_mm_xy(start_vertex.position);
        let (ex, ey) = to_mm_xy(end_vertex.position);
        let (px, py) = perpendicular_unit(ex - sx, ey - sy);

        for bead in 0..effective_count {
            // Per bead index, prefer this endpoint's own strategy output;
            // when this endpoint's Beading ran out (the degenerate case
            // above), fall back to the OTHER endpoint's own real
            // strategy-computed value for that index rather than fabricating
            // a synthetic geometric width. Both vectors are still
            // strategy-sourced in every case — this only decides which
            // endpoint's real output to read for a bead index one side
            // couldn't produce.
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

            let bead_idx = bead as u32;

            let j_start = ExtrusionJunction {
                p: Point3WithWidth {
                    x: sx + px * loc_start_mm,
                    y: sy + py * loc_start_mm,
                    z: 0.0,
                    width: width_start_mm,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                perimeter_index: bead_idx,
            };
            let j_end = ExtrusionJunction {
                p: Point3WithWidth {
                    x: ex + px * loc_end_mm,
                    y: ey + py * loc_end_mm,
                    z: 0.0,
                    width: width_end_mm,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                perimeter_index: bead_idx,
            };

            let is_closed = dist_sq_xy(j_start.p, j_end.p) <= CLOSURE_EPS_MM * CLOSURE_EPS_MM;

            buckets.entry(bead_idx).or_default().push(ExtrusionLine {
                junctions: vec![j_start, j_end],
                inset_idx: bead_idx,
                is_odd: bead_idx % 2 == 1,
                is_closed,
            });
        }
    }

    // `BTreeMap::into_values()` yields buckets in ascending key order, so the
    // outer Vec is already sorted by inset_idx ascending — no explicit sort
    // step needed (and none would be observable as a behavior difference).
    buckets.into_values().collect()
}
