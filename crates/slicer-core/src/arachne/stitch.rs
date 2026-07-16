// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp
// (`WallToolPaths::stitchToolPaths`, calling into
// `Arachne::PolylineStitcher::stitch`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Packet 112 (Track B, T-225): joins open `ExtrusionLine` polylines produced
//! by the Arachne beading-strategy stack across small gaps left by
//! `SkeletalTrapezoidation::generateToolpaths`'s `connectJunctions` pass.
//!
//! Mirrors OrcaSlicer's `WallToolPaths::stitchToolPaths` /
//! `Arachne::PolylineStitcher::stitch`: greedily nearest-endpoint-joins open
//! (`is_closed == false`) lines within `max_gap` of each other, grouped by
//! `(inset_idx, is_odd)` so joins never cross perimeter rings or bead-parity
//! classes. A joined chain whose own two endpoints end up within `max_gap`
//! becomes a closed loop (`is_closed = true`), duplicating the start junction
//! onto the end so `first.xy == last.xy` per the `is_closed()` convention used
//! elsewhere in this crate (see `ExtrusionPath3D::is_closed`).
//!
//! # AC-6 invariant
//!
//! Any line that arrives already closed (`is_closed == true`) is never a
//! candidate for joining — this holds for every inset, not just the primary
//! (inset 0) contour, matching the OrcaSlicer source note that there is no
//! inset-0-specific branch in `stitchToolPaths`. Such lines are moved
//! straight to the output, untouched, so a primary outer-wall loop
//! (`is_closed == true && inset_idx == 0`) is returned byte-identical.
//!
//! # Determinism
//!
//! Grouping uses a `BTreeMap<(u32, bool), _>` key (both integer types, so no
//! float hashing/ordering hazard). Within a group, the greedy nearest-pair
//! search breaks distance ties by `(chain_index, chain_index, endpoint_rank,
//! endpoint_rank)` — a total order over integers — never by iterating a
//! `HashMap` keyed on floating point gap distance.

use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};
use std::collections::BTreeMap;

/// Which end of a chain (first or last junction) an endpoint refers to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Endpoint {
    Start,
    End,
}

impl Endpoint {
    /// Deterministic tie-break rank: `Start` sorts before `End`.
    const fn rank(self) -> u8 {
        match self {
            Endpoint::Start => 0,
            Endpoint::End => 1,
        }
    }
}

/// Joins open (`is_closed == false`) `ExtrusionLine` polylines whose
/// endpoints lie within `max_gap` (millimeters, matching `Point3WithWidth`'s
/// coordinate unit) of each other.
///
/// Lines that arrive already closed are passed through untouched (AC-6).
/// Open lines are grouped by `(inset_idx, is_odd)` and greedily joined
/// nearest-endpoint-first; a joined chain whose own endpoints end up within
/// `max_gap` is marked closed and gains a duplicated closing junction.
pub fn stitch_extrusions(lines: Vec<ExtrusionLine>, max_gap: f64) -> Vec<ExtrusionLine> {
    let mut output: Vec<ExtrusionLine> = Vec::with_capacity(lines.len());
    let mut open_groups: BTreeMap<(u32, bool), Vec<Vec<ExtrusionJunction>>> = BTreeMap::new();

    for line in lines {
        if line.is_closed || line.junctions.len() < 2 {
            // AC-6: already-closed lines (of any inset) are never split or
            // merged. Degenerate (<2 junction) lines have no endpoints to
            // join either — pass both through unchanged.
            output.push(line);
        } else {
            open_groups
                .entry((line.inset_idx, line.is_odd))
                .or_default()
                .push(line.junctions);
        }
    }

    for ((inset_idx, is_odd), group) in open_groups {
        let stitched = stitch_group(group, max_gap, is_odd);
        output.extend(
            stitched
                .into_iter()
                .map(|junctions| finalize_chain(junctions, inset_idx, is_odd, max_gap)),
        );
    }

    output
}

/// XY-only distance (matches `ExtrusionPath3D::is_closed`'s XY-only
/// convention — Arachne toolpaths are a per-layer 2D construct).
fn dist_sq_xy(a: Point3WithWidth, b: Point3WithWidth) -> f64 {
    let dx = (a.x - b.x) as f64;
    let dy = (a.y - b.y) as f64;
    dx * dx + dy * dy
}

fn endpoint_pos(chain: &[ExtrusionJunction], endpoint: Endpoint) -> Point3WithWidth {
    match endpoint {
        Endpoint::Start => chain.first().expect("chain has >=2 junctions").p,
        Endpoint::End => chain.last().expect("chain has >=2 junctions").p,
    }
}

/// Greedily nearest-endpoint-joins all chains in `group` within `max_gap`,
/// merging repeatedly until no candidate pair remains under the threshold.
fn stitch_group(
    mut chains: Vec<Vec<ExtrusionJunction>>,
    max_gap: f64,
    is_odd: bool,
) -> Vec<Vec<ExtrusionJunction>> {
    let max_gap_sq = max_gap * max_gap;

    loop {
        // Candidate: (chain_i, chain_j, endpoint_i, endpoint_j, dist_sq).
        let mut best: Option<(usize, usize, Endpoint, Endpoint, f64)> = None;

        for i in 0..chains.len() {
            for j in (i + 1)..chains.len() {
                for &ei in &[Endpoint::Start, Endpoint::End] {
                    for &ej in &[Endpoint::Start, Endpoint::End] {
                        // canReverse parity gate (OrcaSlicer
                        // PolylineStitcher::stitch): even (`!is_odd`) lines
                        // must never be joined by reversing a chain, so the
                        // same-side endpoint pairs (E,E) and (S,S) — which
                        // require a reversal — are skipped. Odd lines retain
                        // the full 4-way merge.
                        if !is_odd && ei == ej {
                            continue;
                        }
                        let d2 =
                            dist_sq_xy(endpoint_pos(&chains[i], ei), endpoint_pos(&chains[j], ej));
                        if d2 > max_gap_sq {
                            continue;
                        }
                        let candidate = (i, j, ei, ej, d2);
                        best = Some(match best {
                            None => candidate,
                            Some(current) => pick_better(candidate, current),
                        });
                    }
                }
            }
        }

        match best {
            None => break,
            Some((i, j, ei, ej, _)) => {
                let chain_j = chains.remove(j);
                let chain_i = std::mem::take(&mut chains[i]);
                chains[i] = merge_chains(chain_i, ei, chain_j, ej);
            }
        }
    }

    chains
}

/// Deterministic "is `a` strictly better (smaller distance, then smaller
/// index tuple) than `b`" comparison. Never hashes a float; ties are broken
/// purely on integer chain/endpoint ranks.
fn pick_better(
    a: (usize, usize, Endpoint, Endpoint, f64),
    b: (usize, usize, Endpoint, Endpoint, f64),
) -> (usize, usize, Endpoint, Endpoint, f64) {
    match a.4.partial_cmp(&b.4) {
        Some(std::cmp::Ordering::Less) => a,
        Some(std::cmp::Ordering::Greater) => b,
        Some(std::cmp::Ordering::Equal) | None => {
            let a_key = (a.0, a.1, a.2.rank(), a.3.rank());
            let b_key = (b.0, b.1, b.2.rank(), b.3.rank());
            if a_key <= b_key {
                a
            } else {
                b
            }
        }
    }
}

/// Concatenates `chain_i`/`chain_j` at the matched endpoints, reversing
/// whichever side is needed so the join is tail-to-head. The gap between the
/// two matched junctions is preserved as-is (both junctions are kept) rather
/// than collapsed — the joined line still has a visible (sub-`max_gap`) gap
/// segment, matching `PolylineStitcher`'s behavior of bridging rather than
/// snapping endpoints together.
fn merge_chains(
    mut chain_i: Vec<ExtrusionJunction>,
    ei: Endpoint,
    mut chain_j: Vec<ExtrusionJunction>,
    ej: Endpoint,
) -> Vec<ExtrusionJunction> {
    match (ei, ej) {
        (Endpoint::End, Endpoint::Start) => {
            chain_i.extend(chain_j);
            chain_i
        }
        (Endpoint::End, Endpoint::End) => {
            chain_j.reverse();
            chain_i.extend(chain_j);
            chain_i
        }
        (Endpoint::Start, Endpoint::Start) => {
            chain_i.reverse();
            chain_i.extend(chain_j);
            chain_i
        }
        (Endpoint::Start, Endpoint::End) => {
            chain_j.extend(chain_i);
            chain_j
        }
    }
}

/// Finalizes a (possibly merged) chain into an `ExtrusionLine`: if the
/// chain's own two endpoints are within `max_gap`, close the loop
/// (duplicating the start junction onto the end when they don't already
/// coincide exactly) and set `is_closed = true`.
///
/// Tiny-polygon non-closure rule (OrcaSlicer `PolylineStitcher.hpp:136-141`):
/// a chain is left open (never closed into a loop) when the total polyline
/// length plus the closing-segment distance is `< 3 * max_gap`, or when the
/// chain has `<= 2` junctions. Such small/short polylines may still extend
/// into a longer open polyline later in the pipeline and must not be folded
/// into a tiny closed loop.
fn finalize_chain(
    mut junctions: Vec<ExtrusionJunction>,
    inset_idx: u32,
    is_odd: bool,
    max_gap: f64,
) -> ExtrusionLine {
    let is_closed = if junctions.len() >= 2 {
        let start = junctions.first().expect("checked len >= 2").p;
        let end = junctions.last().expect("checked len >= 2").p;
        let closing_dist = dist_sq_xy(start, end).sqrt();
        // Compute chain_length = sum of Euclidean XY distances between
        // consecutive junctions along the polyline (OrcaSlicer
        // chain.polylineLength() + delta, in scaled-mm units).
        let chain_length: f64 = junctions
            .windows(2)
            .map(|w| dist_sq_xy(w[0].p, w[1].p).sqrt())
            .sum();
        // Tiny-poly rule (canonical `PolylineStitcher::stitch`,
        // `PolylineStitcher.hpp`): if the total polyline length + closing-
        // segment distance is < 3 * max_gap, OR the chain has <= 2 junctions,
        // do not close (it might still extend into a longer polyline;
        // 2-vertex polygons are also rejected).
        //
        // `max_gap`, `chain_length` and `closing_dist` are all in MILLIMETERS —
        // `Point3WithWidth`'s coordinate unit, this function's `max_gap`
        // contract, and what production (`arachne/pipeline.rs`) passes. Do NOT
        // reintroduce a `/ UNITS_PER_MM` here: a previous revision did, which
        // silently shrank this threshold from 1.2mm to 0.00012mm so the rule
        // never fired and small fragments closed prematurely — then escaped
        // `remove_small_lines`, which exempts `is_closed` lines. See
        // D-147-STITCH-TINY-POLY-UNITS.
        if chain_length + closing_dist < 3.0 * max_gap || junctions.len() <= 2 {
            false
        } else {
            closing_dist <= max_gap
        }
    } else {
        false
    };

    if is_closed {
        let start = junctions.first().expect("checked len >= 2").p;
        let end = junctions.last().expect("checked len >= 2").p;
        if start.x != end.x || start.y != end.y {
            let closing = junctions.first().cloned().expect("checked len >= 2");
            junctions.push(closing);
        }
    }

    ExtrusionLine {
        junctions,
        inset_idx,
        is_odd,
        is_closed,
    }
}
