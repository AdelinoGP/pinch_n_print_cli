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
//! `inset_idx` (canonical stitches per inset bucket) with per-pair
//! `canConnect` (same odd-ness) and `canReverse(nearby)` (no reversal of
//! even chains) gates. A joined chain whose own two endpoints end up within
//! `max_gap` becomes a closed loop (`is_closed = true`), duplicating the
//! start junction onto the end so `first.xy == last.xy` per the `is_closed()`
//! convention used elsewhere in this crate (see `ExtrusionPath3D::is_closed`).
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
//! Grouping uses a `BTreeMap<u32, _>` key (integer type, so no float
//! hashing/ordering hazard). Within a group, the greedy nearest-pair
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
/// Open lines are grouped by `inset_idx` and greedily joined
/// nearest-endpoint-first within each inset group; a joined chain whose own
/// endpoints end up within `max_gap` is marked closed and gains a duplicated
/// closing junction.
///
/// Within a group, canonical `canConnect` (same odd-ness) is enforced per
/// candidate pair — never by pre-splitting the group — and canonical
/// `canReverse(nearby)` gates reversal joins for even nearby chains (see
/// `stitch_group`).
pub fn stitch_extrusions(lines: Vec<ExtrusionLine>, max_gap: f64) -> Vec<ExtrusionLine> {
    let mut output: Vec<ExtrusionLine> = Vec::with_capacity(lines.len());
    // Group by inset only (NOT by odd-ness): canonical `canConnect`
    // requires equal odd-ness per join, but the join search itself runs
    // over the whole inset bucket — an even chain may still legally extend
    // through an odd chain's endpoint region when the geometry lines up.
    // (The per-pair odd gate below keeps the parity rule; grouping must
    // not pre-split the bucket or cross-parity continuations are never
    // even considered. The l29 inner-contour split: the stern bead-3 line
    // and the cross bead-3 line share inset 3 but differ in is_odd, so the
    // old grouping put them in different buckets and the 0.00mm End/Start
    // join was never attempted.)
    let mut open_groups: BTreeMap<u32, Vec<(Vec<ExtrusionJunction>, bool)>> =
        BTreeMap::new();

    for line in lines {
        if line.is_closed || line.junctions.len() < 2 {
            // AC-6: already-closed lines (of any inset) are never split or
            // merged. Degenerate (<2 junction) lines have no endpoints to
            // join either — pass both through unchanged.
            output.push(line);
        } else {
            open_groups
                .entry(line.inset_idx)
                .or_default()
                .push((line.junctions, line.is_odd));
        }
    }

    for (inset_idx, group) in open_groups {
        let stitched = stitch_group(group, max_gap);
        output.extend(stitched.into_iter().map(|chain| {
            if chain.closed {
                close_chain(chain.junctions, inset_idx, chain.is_odd)
            } else {
                finalize_chain(chain.junctions, inset_idx, chain.is_odd, max_gap)
            }
        }));
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

/// Canonical `PolylineStitcher::stitch`'s bias on a closing candidate
/// (`scaled<coord_t>(0.01)`, in millimetres): added for a chain that should
/// not close (prefer continuing the polyline), subtracted for one that should.
const CLOSING_BIAS_MM: f64 = 0.01;

/// A chain under construction in [`stitch_group`].
struct Chain {
    junctions: Vec<ExtrusionJunction>,
    is_odd: bool,
    /// Canonical `should_close`: starts as `isOdd(line)` and is cleared once
    /// the chain absorbs another odd line (`should_close & !isOdd(closest)`).
    should_close: bool,
    /// Set once the chain has been closed on itself; it takes no further part
    /// in joining.
    closed: bool,
}

/// The next stitching step: join two chains at the given endpoints, or close
/// one chain on itself.
#[derive(Clone, Copy, Debug)]
enum Step {
    Join(usize, usize, Endpoint, Endpoint),
    Close(usize),
}

impl Step {
    /// Deterministic integer tie-break key: joins by chain/endpoint ranks, a
    /// closing after every join that starts at the same chain index.
    fn key(self) -> (usize, usize, u8, u8) {
        match self {
            Step::Join(i, j, ei, ej) => (i, j, ei.rank(), ej.rank()),
            Step::Close(i) => (i, usize::MAX, 2, 2),
        }
    }
}

fn polyline_length(junctions: &[ExtrusionJunction]) -> f64 {
    junctions
        .windows(2)
        .map(|w| dist_sq_xy(w[0].p, w[1].p).sqrt())
        .sum()
}

/// Greedily nearest-endpoint-joins all chains in `group` within `max_gap`,
/// merging repeatedly until no candidate step remains under the threshold.
///
/// Closing a chain on itself competes with joining it to another chain, as
/// in canonical `PolylineStitcher::stitch`: a chain's own start is a closing
/// candidate for its end when they are within `max_gap`, unless the closed
/// loop would be tiny (`chain_length + dist < 3 * max_gap`, or `<= 2`
/// junctions). The candidate's distance carries canonical's
/// [`CLOSING_BIAS_MM`]. Without the closing candidate, a loop that already
/// met itself was joined to any other same-inset loop within `max_gap` (for
/// example the two inner beads of a ring about four beads thick), printing a
/// chord between them.
///
/// Each chain carries its own odd-ness (`is_odd` travels with the junctions:
/// a merged chain is odd only while every segment merged into it was odd —
/// matching canonical `should_close = should_close & !isOdd(newcomer)`).
fn stitch_group(chains: Vec<(Vec<ExtrusionJunction>, bool)>, max_gap: f64) -> Vec<Chain> {
    let max_gap_sq = max_gap * max_gap;
    let mut chains: Vec<Chain> = chains
        .into_iter()
        .map(|(junctions, is_odd)| Chain {
            junctions,
            is_odd,
            should_close: is_odd,
            closed: false,
        })
        .collect();

    loop {
        // Candidate: (step, effective distance in mm).
        let mut best: Option<(Step, f64)> = None;
        let mut offer = |step: Step, dist: f64| {
            best = Some(match best {
                None => (step, dist),
                Some(current) => pick_better((step, dist), current),
            });
        };

        for i in 0..chains.len() {
            if chains[i].closed {
                continue;
            }
            // Closing candidate (canonical: the nearby point is the chain's
            // own front).
            {
                let chain = &chains[i];
                let start = endpoint_pos(&chain.junctions, Endpoint::Start);
                let end = endpoint_pos(&chain.junctions, Endpoint::End);
                let dist = dist_sq_xy(start, end).sqrt();
                let tiny = polyline_length(&chain.junctions) + dist < 3.0 * max_gap
                    || chain.junctions.len() <= 2;
                if dist <= max_gap && !tiny {
                    let biased = if chain.should_close {
                        dist - CLOSING_BIAS_MM
                    } else {
                        dist + CLOSING_BIAS_MM
                    };
                    offer(Step::Close(i), biased);
                }
            }
            for j in (i + 1)..chains.len() {
                if chains[j].closed {
                    continue;
                }
                // canConnect (OrcaSlicer PolylineStitcher::stitch): only
                // same-parity chains may join.
                if chains[i].is_odd != chains[j].is_odd {
                    continue;
                }
                let is_odd = chains[i].is_odd;
                for &ei in &[Endpoint::Start, Endpoint::End] {
                    for &ej in &[Endpoint::Start, Endpoint::End] {
                        // canReverse parity gate (OrcaSlicer
                        // PolylineStitcher::stitch, `canReverse(nearby)`):
                        // `canReverse(nearby)` is true exactly for odd lines:
                        // an even nearby chain can only extend the chain
                        // without flipping — joins that would reverse it,
                        // (End, End) and (Start, Start), are rejected. Odd
                        // nearby chains keep the full 4-way merge. The old
                        // gate (`ei == ej` skip for even groups) confused
                        // endpoint-side symmetry with reversal and rejected
                        // even End-to-Start joins — the one legal
                        // continuation — stranding every even open chain pair
                        // whose facing ends were End/Start (the l29
                        // inner-contour split: two bead-3 lines head-to-tail
                        // 0.00mm apart, never joined, never closed).
                        //
                        // Chain-role mapping: `chains[i]` is the chain under
                        // extension, `chains[j]` the nearby candidate. The
                        // merge keeps `chains[i]`'s direction: (End, Start)
                        // and (Start, End) joins need no reversal; (End, End)
                        // and (Start, Start) reverse the nearby chain.
                        if !is_odd {
                            let nearby_reversed = matches!(
                                (ei, ej),
                                (Endpoint::End, Endpoint::End) | (Endpoint::Start, Endpoint::Start)
                            );
                            if nearby_reversed {
                                continue;
                            }
                        }
                        let d2 = dist_sq_xy(
                            endpoint_pos(&chains[i].junctions, ei),
                            endpoint_pos(&chains[j].junctions, ej),
                        );
                        if d2 > max_gap_sq {
                            continue;
                        }
                        offer(Step::Join(i, j, ei, ej), d2.sqrt());
                    }
                }
            }
        }

        match best {
            None => break,
            Some((Step::Close(i), _)) => chains[i].closed = true,
            Some((Step::Join(i, j, ei, ej), _)) => {
                let chain_j = chains.remove(j);
                debug_assert_eq!(
                    chains[i].is_odd, chain_j.is_odd,
                    "canConnect gate passed, odd-ness must match"
                );
                let chain_i = std::mem::take(&mut chains[i].junctions);
                chains[i].junctions = merge_chains(chain_i, ei, chain_j.junctions, ej);
                chains[i].should_close = chains[i].should_close && !chain_j.is_odd;
            }
        }
    }

    chains
}

/// Deterministic "is `a` strictly better (smaller distance, then smaller
/// integer key) than `b`" comparison. Never hashes a float; ties are broken
/// purely on integer chain/endpoint ranks.
fn pick_better(a: (Step, f64), b: (Step, f64)) -> (Step, f64) {
    match a.1.partial_cmp(&b.1) {
        Some(std::cmp::Ordering::Less) => a,
        Some(std::cmp::Ordering::Greater) => b,
        Some(std::cmp::Ordering::Equal) | None => {
            if a.0.key() <= b.0.key() {
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

/// Emits a chain that [`stitch_group`] closed on itself: appends the start
/// junction when the two ends do not already coincide (canonical
/// `stitchToolPaths` reconnects closed polygons whose endpoints differ by
/// less than the stitch distance) and marks the line closed.
fn close_chain(
    mut junctions: Vec<ExtrusionJunction>,
    inset_idx: u32,
    is_odd: bool,
) -> ExtrusionLine {
    let start = junctions.first().expect("closed chain has >= 3 junctions").p;
    let end = junctions.last().expect("closed chain has >= 3 junctions").p;
    if start.x != end.x || start.y != end.y {
        let closing = junctions.first().cloned().expect("checked non-empty");
        junctions.push(closing);
    }
    ExtrusionLine {
        junctions,
        inset_idx,
        is_odd,
        is_closed: true,
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
        // `remove_small_lines`, which exempts `is_closed` lines.
        // See the unit contract documented above.
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
