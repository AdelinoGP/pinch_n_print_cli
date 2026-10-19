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
//! Ports canonical `Arachne::PolylineStitcher::stitch`
//! (`Arachne/utils/PolylineStitcher.hpp`, called per wall by
//! `WallToolPaths::stitchToolPaths`). The walk is **per line**, not global
//! nearest-pair: each unprocessed line in line order seeds a chain, and that
//! chain is extended from its back only, absorbing the nearest eligible
//! candidate endpoint within `max_gap` until no candidate remains. A chain is
//! never re-seeded and a candidate line is never absorbed by two chains
//! (`processed[]`).
//!
//! Per seed, canonical makes two passes (`go_in_reverse_direction`): the chain
//! is extended forward, then reversed and extended from its other end. The
//! chain is closed when a candidate point within `snap_distance` of the
//! chain's **front** wins the nearest-candidate search; that candidate's
//! distance carries canonical's ±0.01 mm closing bias, and the tiny-poly gate
//! (`chain_length + dist < 3 * max_gap`, or `<= 2` junctions) rejects closing
//! a chain that could still grow. A join closer than `snap_distance` drops the
//! duplicated endpoint junction (canonical `++start_pos`) instead of keeping
//! both copies.
//!
//! Canonical's `canConnect` (equal odd-ness) and `canReverse` (odd lines only)
//! gates are enforced per candidate pair. `should_close` starts as the seed's
//! `is_odd` and is cleared as soon as an odd line is absorbed
//! (`should_close & !isOdd(closest)`).
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
//! hashing/ordering hazard). Canonical's nearest-candidate search walks a
//! spatial hash grid, whose iteration order is unspecified; this port scans
//! the candidate endpoints in `(line_idx, endpoint_rank)` order and keeps the
//! strict minimum (`dist < closest_distance`), so equal distances resolve to
//! the lowest `(line_idx, endpoint_rank)` — a total order over integers,
//! never float hashing. This differs from canonical only when two eligible
//! candidates are at the *same* distance, and canonical's own choice there is
//! grid-order dependent.
//!
//! Canonical's "good enough next line" early-exit (`return false` once a
//! candidate's biased distance is below `snap_distance`) is reproduced: the
//! scan stops at the first such candidate in `(line_idx, endpoint_rank)`
//! order. Canonical decides which candidate that is by hash-grid iteration
//! order, so the choice was already arbitrary; this port makes it
//! deterministic. The only candidates the early exit can select are
//! sub-`snap_distance + CLOSING_BIAS_MM` (0.02 mm) from the query point, so
//! the difference between two such candidates is a sub-snap duplicate-junction
//! choice.

use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};
use std::collections::BTreeMap;

/// Canonical `snap_distance` default (`scaled<coord_t>(0.01)`, millimetres):
/// points closer than this are the same point. A join whose `segment_dist` is
/// below it drops the duplicated endpoint junction (canonical `++start_pos`).
const SNAP_DISTANCE_MM: f64 = 0.01;

/// Canonical closing bias (`scaled<coord_t>(0.01)`, millimetres): added to a
/// closing candidate's distance when the chain should not close (prefer
/// continuing the polyline over closing a polygon), subtracted when it should
/// (a chain of 100 % even lines prefers to close).
const CLOSING_BIAS_MM: f64 = 0.01;

/// XY-only distance (matches `ExtrusionPath3D::is_closed`'s XY-only
/// convention — Arachne toolpaths are a per-layer 2D construct).
fn dist_sq_xy(a: Point3WithWidth, b: Point3WithWidth) -> f64 {
    let dx = (a.x - b.x) as f64;
    let dy = (a.y - b.y) as f64;
    dx * dx + dy * dy
}

fn dist_xy(a: Point3WithWidth, b: Point3WithWidth) -> f64 {
    dist_sq_xy(a, b).sqrt()
}

/// Canonical `ExtrusionLine::polylineLength()`: sum of the lengths of the
/// consecutive junction segments.
fn polyline_length(junctions: &[ExtrusionJunction]) -> f64 {
    junctions.windows(2).map(|w| dist_xy(w[0].p, w[1].p)).sum()
}

/// An open line under stitching: its junctions and its odd-ness, which travels
/// with the line (canonical `ExtrusionLine::is_odd`).
type Candidate = (Vec<ExtrusionJunction>, bool);

/// A chain produced by [`stitch_inset`].
struct Chain {
    junctions: Vec<ExtrusionJunction>,
    is_odd: bool,
    /// Set when the nearest-candidate search selected a closing candidate
    /// (canonical `closest_is_closing_polygon`).
    closed: bool,
}

/// Joins open (`is_closed == false`) `ExtrusionLine` polylines whose
/// endpoints lie within `max_gap` (millimeters, matching `Point3WithWidth`'s
/// coordinate unit) of each other.
///
/// Lines that arrive already closed are passed through untouched (AC-6);
/// degenerate (<2 junction) lines have no endpoints to join either and are
/// passed through unchanged. Open lines are grouped by `inset_idx` (canonical
/// stitches one wall bucket at a time) and walked per line by
/// [`stitch_inset`], canonical `PolylineStitcher::stitch`'s order.
///
/// Within a bucket the open chains are emitted before the closed polygons, as
/// canonical `WallToolPaths::stitchToolPaths` appends `stitched_polylines`
/// first and `closed_polygons` after.
pub fn stitch_extrusions(lines: Vec<ExtrusionLine>, max_gap: f64) -> Vec<ExtrusionLine> {
    let mut output: Vec<ExtrusionLine> = Vec::with_capacity(lines.len());
    // Group by inset only (NOT by odd-ness): canonical `canConnect`
    // requires equal odd-ness per join, but the join search itself runs
    // over the whole inset bucket — an even chain may still legally extend
    // through an odd chain's endpoint region when the geometry lines up.
    // (The per-pair odd gate keeps the parity rule; grouping must not
    // pre-split the bucket or cross-parity continuations are never even
    // considered.)
    let mut open_groups: BTreeMap<u32, Vec<Candidate>> = BTreeMap::new();

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
        let mut open: Vec<ExtrusionLine> = Vec::new();
        let mut closed: Vec<ExtrusionLine> = Vec::new();
        for chain in stitch_inset(group, max_gap) {
            if chain.closed {
                closed.push(close_chain(
                    chain.junctions,
                    inset_idx,
                    chain.is_odd,
                    max_gap,
                ));
            } else {
                open.push(ExtrusionLine {
                    junctions: chain.junctions,
                    inset_idx,
                    is_odd: chain.is_odd,
                    is_closed: false,
                });
            }
        }
        output.extend(open);
        output.extend(closed);
    }

    output
}

/// Canonical `PolylineStitcher::stitch`'s walk over one inset bucket.
///
/// Each unprocessed line seeds a chain (in line order); the chain absorbs the
/// nearest eligible candidate endpoint from its back, and the walk finishes a
/// seed only after trying the reversed direction. See the module docs for the
/// gates and the one documented divergence (deterministic candidate order).
fn stitch_inset(lines: Vec<Candidate>, max_gap: f64) -> Vec<Chain> {
    let n = lines.len();
    let mut processed = vec![false; n];
    let snap_sq = SNAP_DISTANCE_MM * SNAP_DISTANCE_MM;
    let mut out: Vec<Chain> = Vec::with_capacity(n);

    for seed in 0..n {
        if processed[seed] {
            continue;
        }
        processed[seed] = true;

        let mut chain = lines[seed].0.clone();
        let chain_is_odd = lines[seed].1;
        // Canonical `should_close = isOdd(line)`, cleared by an odd absorb.
        let mut should_close = chain_is_odd;
        let mut closest_is_closing_polygon = false;

        for go_in_reverse_direction in [false, true] {
            if go_in_reverse_direction {
                // Try extending the chain in the other direction.
                chain.reverse();
            }

            loop {
                let from = chain.last().expect("chain has >= 2 junctions").p;
                let front = chain.first().expect("chain has >= 2 junctions").p;
                let chain_length = polyline_length(&chain);
                let chain_len = chain.len();

                // Candidate: (line_idx, endpoint_idx, raw segment distance,
                // is_closing_segment).
                let mut closest: Option<(usize, usize, f64, bool)> = None;
                let mut closest_distance = f64::MAX;

                'candidates: for candidate in 0..n {
                    let candidate_line = &lines[candidate].0;
                    let last = candidate_line.len() - 1;
                    for point_idx in [0usize, last] {
                        let p = candidate_line[point_idx].p;
                        let raw = dist_xy(from, p);
                        if raw > max_gap {
                            continue; // keep looking
                        }

                        let mut dist = raw;
                        let mut is_closing_segment = false;
                        if dist_sq_xy(p, front) < snap_sq {
                            // The candidate is at the chain's front: closing.
                            // Tiny polylines are not closed — they may still
                            // grow into a longer polyline.
                            if chain_length + dist < 3.0 * max_gap || chain_len <= 2 {
                                continue; // look for a better next line
                            }
                            is_closing_segment = true;
                            if should_close {
                                dist -= CLOSING_BIAS_MM; // prefer closing
                            } else {
                                dist += CLOSING_BIAS_MM; // prefer continuing
                            }
                        } else if processed[candidate] {
                            // Already moved to output.
                            continue; // keep looking for a connection
                        }

                        // `canReverse(nearby)` (odd lines only) rejects joins
                        // that would reverse an even nearby line.
                        let nearby_is_odd = lines[candidate].1;
                        let nearby_would_be_reversed = (point_idx != 0) != go_in_reverse_direction;
                        if !nearby_is_odd && nearby_would_be_reversed {
                            continue; // keep looking for a connection
                        }
                        // `canConnect(chain, nearby)`: never mix parities.
                        if chain_is_odd != nearby_is_odd {
                            continue; // keep looking for a connection
                        }

                        if dist < closest_distance {
                            closest_distance = dist;
                            closest = Some((candidate, point_idx, raw, is_closing_segment));
                        }
                        if dist < SNAP_DISTANCE_MM {
                            // We have found a good enough next line: stop
                            // looking for alternatives.
                            break 'candidates;
                        }
                    }
                }

                let Some((candidate, point_idx, segment_dist, is_closing_segment)) = closest else {
                    break; // no next line
                };
                if is_closing_segment {
                    // We closed the polygon.
                    closest_is_closing_polygon = true;
                    break;
                }

                // `segment_dist` is the raw chain.back() → candidate distance
                // (canonical recomputes it after the biased comparison). A
                // join closer than `snap_distance` drops the duplicated
                // junction (canonical `++start_pos`).
                let candidate_line = &lines[candidate].0;
                let snapped = segment_dist < SNAP_DISTANCE_MM;
                if point_idx == 0 {
                    let start = usize::from(snapped);
                    chain.extend_from_slice(&candidate_line[start..]);
                } else {
                    let end = if snapped {
                        candidate_line.len() - 1
                    } else {
                        candidate_line.len()
                    };
                    chain.extend(candidate_line[..end].iter().rev().cloned());
                }
                // If we connect an even to an odd line, we should no longer
                // try to close it.
                should_close = should_close && !lines[candidate].1;
                processed[candidate] = true;
            }

            if closest_is_closing_polygon {
                if go_in_reverse_direction {
                    // Re-reverse to retain the seed's original direction.
                    chain.reverse();
                }
                break; // don't consider the reverse direction
            }
        }

        if closest_is_closing_polygon {
            out.push(Chain {
                junctions: chain,
                is_odd: chain_is_odd,
                closed: true,
            });
        } else {
            // `canReverse(seed)` is `seed.is_odd`: an even chain is not
            // allowed to stay reversed, so restore its direction.
            if !chain_is_odd {
                chain.reverse();
            }
            out.push(Chain {
                junctions: chain,
                is_odd: chain_is_odd,
                closed: false,
            });
        }
    }

    out
}

/// Emits a chain that [`stitch_inset`] closed on itself
/// (`WallToolPaths::stitchToolPaths`): when the endpoints differ by less than
/// `max_gap` the start junction is appended so `first.xy == last.xy` per the
/// `is_closed()` convention used elsewhere in this crate, and the line is
/// marked closed.
fn close_chain(
    mut junctions: Vec<ExtrusionJunction>,
    inset_idx: u32,
    is_odd: bool,
    max_gap: f64,
) -> ExtrusionLine {
    let start = junctions
        .first()
        .expect("closed chain has >= 3 junctions")
        .p;
    let end = junctions.last().expect("closed chain has >= 3 junctions").p;
    let differs = start.x != end.x || start.y != end.y;
    if differs && dist_xy(end, start) < max_gap {
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
