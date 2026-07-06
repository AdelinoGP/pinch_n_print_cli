// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
// (`generateJunctions` L2013-2079, `getBeading`/`getOrCreateBeading` L2029,
// L2091-2127).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Regression locks for three real bugs found in packet 141's first
//! `generate_junctions` implementation (commit `798bb324`), fixed in commit
//! `9367d239`. Each bug was independently confirmed against a fresh
//! OrcaSlicer ground-truth read of `generateJunctions`
//! (`SkeletalTrapezoidation.cpp:2013-2079`), not from the original audit's
//! memory of it — see `docs/DEVIATION_LOG.md`'s `D-141-JUNCTION-BANDS`
//! correction note for the full account.
//!
//! Unlike `arachne_parity_red_junction_bands.rs` (AC-1/AC-2, which run the
//! full pipeline including the domain-chain walk and are currently blocked
//! on packet 142's `connectJunctions` rewrite), these tests call
//! `generate_junctions` directly and are isolated from the chain-walk
//! entirely — they pin `generate_junctions`'s own contract, not the
//! downstream chain-stitching packet 142 owns.
//!
//! Host-only: gated behind `host-algos`, matching the rest of the Arachne
//! junction suite.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::generate_toolpaths::generate_junctions;
use slicer_core::beading::{Beading, BeadingStrategy};
use slicer_core::skeletal_trapezoidation::{
    populate_beading_propagation, EdgeType, RibData, STHalfEdge, STVertex,
    SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::{Vertex, NO_INDEX};
use slicer_ir::UNITS_PER_MM;

/// Deterministic bead-width/location generator (mirrors the equivalent
/// test-local strategy in `generate_toolpaths.rs`'s own `#[cfg(test)]`
/// module and `arachne_junction_upward_half_edge_only.rs`): splits
/// `thickness` into `bead_count` equal-width beads centered within their own
/// slice.
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
        Beading {
            total_thickness: thickness,
            bead_widths: vec![width; bead_count],
            toolpath_locations: (0..bead_count).map(|i| width * (i as f64 + 0.5)).collect(),
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
        bead_count as f64 * 0.4 * UNITS_PER_MM
    }

    fn type_label(&self) -> &'static str {
        "FixedTestStrategy"
    }
}

/// Builds a single central twin-pair edge: `v0` (from, R = 1mm, `bead_count
/// = None`) -> `v1` (to/peak, R = 5mm, `bead_count = Some(2)`). Both
/// half-edges are `central = true`, `edge_type = NORMAL` -- the upward
/// direction (`edges[0]`) is the one `generate_junctions` should resolve a
/// beading for.
fn peak_vs_boundary_fixture() -> SkeletalTrapezoidationGraph {
    let v0 = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 1.0 * UNITS_PER_MM,
        bead_count: None,
        transition_ratio: 0.0,
    };
    let v1 = STVertex {
        position: Vertex {
            x: 10.0 * UNITS_PER_MM,
            y: 0.0,
        },
        distance_to_boundary: 5.0 * UNITS_PER_MM,
        bead_count: Some(2),
        transition_ratio: 0.0,
    };
    let edge_up = STHalfEdge {
        start_vertex: 0,
        twin: 1,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let edge_down = STHalfEdge {
        start_vertex: 1,
        twin: 0,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    SkeletalTrapezoidationGraph {
        vertices: vec![v0, v1],
        edges: vec![edge_up, edge_down],
        centrality_filtered: true,
        rib: RibData::default(),
        beading_propagation: vec![None, None],
    }
}

/// **Bug 1 regression (peak-vs-boundary beading anchor).** Canonical
/// `generateJunctions` resolves ONE beading at the edge's peak
/// (`getOrCreateBeading(edge->to, ...)`, `SkeletalTrapezoidation.cpp:2029`
/// -- the HIGHER-`distance_to_boundary` endpoint), never at the
/// lower-R/boundary-side endpoint. Packet 141's first implementation
/// resolved it at `edge.from` (the lower-R vertex) as the PRIMARY path,
/// falling back to the peak only when the boundary-side entry was empty --
/// backwards from canonical.
///
/// This test directly populates BOTH vertices' `beading_propagation` side-
/// table entries with deliberately different, distinguishable widths (so
/// the side table itself never falls through to a `strategy.compute()`
/// fallback on either side) and asserts the emitted junction's width comes
/// from `v1`'s (peak's) beading, never `v0`'s (boundary-side's).
///
/// FAILS under the pre-fix `get_beding(edge.from)`-primary implementation
/// (which would return `v0`'s 0.05mm width); PASSES under the fixed
/// `get_beding(edge.to)`-primary implementation (returns `v1`'s 0.9mm
/// width).
#[test]
fn generate_junctions_resolves_beading_at_peak_not_boundary_side() {
    let mut graph = peak_vs_boundary_fixture();

    // v0 (boundary-side, R=1mm): a deliberately WRONG-if-used single-bead
    // beading with a tiny, distinctive width.
    graph.beading_propagation[0] = Some(Beading {
        total_thickness: 2.0 * UNITS_PER_MM,
        bead_widths: vec![0.05 * UNITS_PER_MM],
        toolpath_locations: vec![1.0 * UNITS_PER_MM],
        left_over: 0.0,
    });
    // v1 (peak, R=5mm): the CORRECT beading to resolve, with a distinctive
    // width that cannot be confused with v0's.
    graph.beading_propagation[1] = Some(Beading {
        total_thickness: 10.0 * UNITS_PER_MM,
        bead_widths: vec![0.9 * UNITS_PER_MM, 1.1 * UNITS_PER_MM],
        toolpath_locations: vec![1.0 * UNITS_PER_MM, 3.0 * UNITS_PER_MM],
        left_over: 0.0,
    });

    let strategy = FixedBeadingStrategy;
    let junctions = generate_junctions(&graph, &strategy);

    let (from_j, _to_j) = junctions.get(&0).unwrap_or_else(|| {
        panic!(
            "upward edge (edges[0]) must have an entry in edge_junctions; got keys = {:?}",
            junctions.keys().collect::<Vec<_>>()
        )
    });
    assert!(
        !from_j.is_empty(),
        "expected at least one in-band bead for the upward edge"
    );

    let emitted_width_mm = from_j[0].p.width;
    assert!(
        (emitted_width_mm - 0.9).abs() < 1e-3,
        "emitted junction width is {emitted_width_mm}mm, expected ~0.9mm (v1/peak's bead_widths[0]). \
         A width near 0.05mm would mean generate_junctions resolved the beading at v0 (the \
         boundary-side/lower-R vertex) instead of v1 (the peak) -- the exact regression this test \
         guards against (bug 1 of D-141-JUNCTION-BANDS's correction note)."
    );
    assert!(
        (emitted_width_mm - 0.05).abs() > 1e-3,
        "emitted junction width ({emitted_width_mm}mm) matches v0's (boundary-side) bead width -- \
         generate_junctions resolved the WRONG vertex's beading (bug 1 regression)."
    );
}

/// **Bug 2 regression (width source: resolved beading's own array vs. an ad
/// hoc per-bead recompute).** Canonical writes each emitted junction's width
/// directly from the ONE resolved beading's own array
/// (`beading->bead_widths[junction_idx]`, `SkeletalTrapezoidation.cpp:2076`).
/// Packet 141's first implementation instead called
/// `strategy.compute(2 * from_r_or_to_r, bead_count).bead_widths.first()`
/// PER BEAD -- always index 0 of a FRESH computation, regardless of the
/// bead's own index `idx`, and regardless of the resolved (peak) beading's
/// own array.
///
/// This test builds a fixture with TWO in-band beads at the SAME edge whose
/// resolved peak beading has genuinely different widths at index 0 and
/// index 1, and asserts the two emitted junctions' widths are DISTINCT and
/// match `bead_widths[0]`/`bead_widths[1]` respectively.
///
/// FAILS under the pre-fix per-bead-recompute implementation (which would
/// give BOTH junctions the identical width -- `strategy.compute(2*r,
/// bead_count).bead_widths.first()` does not vary with `idx`, and `from_r`/
/// `to_r` are fixed per edge); PASSES under the fixed
/// `beading.bead_widths[idx]` implementation (the two widths differ,
/// matching the resolved beading's own array exactly).
#[test]
fn generate_junctions_reads_width_from_beadings_own_array_per_bead_index() {
    let v0 = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 0.5 * UNITS_PER_MM,
        bead_count: None,
        transition_ratio: 0.0,
    };
    let v1 = STVertex {
        position: Vertex {
            x: 20.0 * UNITS_PER_MM,
            y: 0.0,
        },
        distance_to_boundary: 10.0 * UNITS_PER_MM,
        bead_count: Some(3),
        transition_ratio: 0.0,
    };
    let edge_up = STHalfEdge {
        start_vertex: 0,
        twin: 1,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let edge_down = STHalfEdge {
        start_vertex: 1,
        twin: 0,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: true,
        edge_type: EdgeType::NORMAL,
        ..STHalfEdge::default()
    };
    let mut graph = SkeletalTrapezoidationGraph {
        vertices: vec![v0, v1],
        edges: vec![edge_up, edge_down],
        centrality_filtered: true,
        rib: RibData::default(),
        beading_propagation: vec![None, None],
    };

    // Peak (v1)'s resolved beading: 3 beads with genuinely distinct widths,
    // all within the edge's [0.5mm, 10mm] radius band.
    graph.beading_propagation[1] = Some(Beading {
        total_thickness: 20.0 * UNITS_PER_MM,
        bead_widths: vec![
            0.3 * UNITS_PER_MM,
            0.6 * UNITS_PER_MM,
            0.9 * UNITS_PER_MM,
        ],
        toolpath_locations: vec![
            1.0 * UNITS_PER_MM,
            5.0 * UNITS_PER_MM,
            9.0 * UNITS_PER_MM,
        ],
        left_over: 0.0,
    });

    let strategy = FixedBeadingStrategy;
    let junctions = generate_junctions(&graph, &strategy);

    let (from_j, _to_j) = junctions.get(&0).unwrap_or_else(|| {
        panic!(
            "upward edge (edges[0]) must have an entry in edge_junctions; got keys = {:?}",
            junctions.keys().collect::<Vec<_>>()
        )
    });
    assert_eq!(
        from_j.len(),
        2,
        "expected exactly 2 in-band beads (indices 0 and 1; index 2's location, 9mm, exceeds \
         nothing but its width must differ from the others -- this fixture's from_r=0.5mm keeps \
         indices 0 and 1 in-band and the scan's mid-start (index 1) plus outward walk should \
         reach both), got {} junction(s)",
        from_j.len()
    );

    let width_bead0_mm = from_j[0].p.width;
    let width_bead1_mm = from_j[1].p.width;

    assert!(
        (width_bead0_mm - 0.3).abs() < 1e-3,
        "bead 0's emitted width is {width_bead0_mm}mm, expected ~0.3mm (the resolved peak \
         beading's own bead_widths[0])."
    );
    assert!(
        (width_bead1_mm - 0.6).abs() < 1e-3,
        "bead 1's emitted width is {width_bead1_mm}mm, expected ~0.6mm (the resolved peak \
         beading's own bead_widths[1])."
    );
    assert!(
        (width_bead0_mm - width_bead1_mm).abs() > 1e-3,
        "bead 0 and bead 1 emitted the SAME width ({width_bead0_mm}mm) -- this is the exact \
         signature of the pre-fix bug: `strategy.compute(2 * from_r_or_to_r, bead_count)\
         .bead_widths.first()` recomputes a fresh beading per bead call but always reads index \
         0, so every bead on the same edge collapses to the same two (from-side, to-side) \
         values regardless of its own index (bug 2 of D-141-JUNCTION-BANDS's correction note). \
         The fixed implementation reads `bead_widths[idx]` directly from the ONE resolved peak \
         beading, so different bead indices genuinely differ."
    );
}

/// **Bug 3 regression (ribs / non-central edges must not be excluded).**
/// Canonical `generateJunctions` has NO `edge->data.type` or centrality
/// check anywhere in its loop (`SkeletalTrapezoidation.cpp:2015-2079`) --
/// ribs (`EXTRA_VD`) are genuine junction carriers, not excluded. Packet
/// 141's first implementation retained `if !edge.central { continue }` and
/// `if edge.edge_type == EdgeType::EXTRA_VD { continue }`, silently
/// dropping every rib from consideration.
///
/// This test builds a minimal rib pair (`back` = boundary(R=0) -> spine
/// (R=5mm), the upward/contributing direction; `forth` = the reverse,
/// downward, correctly excluded) with `central = false` and
/// `edge_type = EXTRA_VD` on both halves -- exactly the shape
/// `graph.rs::Builder::make_rib` produces -- and asserts the rib's upward
/// half gets a non-empty `edge_junctions` entry.
///
/// FAILS under the pre-fix type/centrality-gated implementation (which
/// would `continue` past both rib halves, leaving NO entry for either);
/// PASSES under the fixed implementation (the upward rib half resolves a
/// real in-band bead from the spine vertex's beading).
#[test]
fn generate_junctions_does_not_exclude_ribs() {
    let boundary = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 0.0,
        bead_count: None,
        transition_ratio: 0.0,
    };
    let spine = STVertex {
        position: Vertex {
            x: 5.0 * UNITS_PER_MM,
            y: 0.0,
        },
        distance_to_boundary: 5.0 * UNITS_PER_MM,
        bead_count: Some(2),
        transition_ratio: 0.0,
    };
    // `back`: boundary -> spine, the UPWARD (contributing) rib half.
    let rib_back = STHalfEdge {
        start_vertex: 0,
        twin: 1,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: false,
        edge_type: EdgeType::EXTRA_VD,
        ..STHalfEdge::default()
    };
    // `forth`: spine -> boundary, the DOWNWARD half (correctly excluded by
    // the upward-only selection, independent of the type/centrality gate
    // this test targets).
    let rib_forth = STHalfEdge {
        start_vertex: 1,
        twin: 0,
        next: NO_INDEX,
        prev: NO_INDEX,
        central: false,
        edge_type: EdgeType::EXTRA_VD,
        ..STHalfEdge::default()
    };
    let mut graph = SkeletalTrapezoidationGraph {
        vertices: vec![boundary, spine],
        edges: vec![rib_back, rib_forth],
        centrality_filtered: true,
        rib: RibData::default(),
        beading_propagation: vec![None, None],
    };

    let strategy = FixedBeadingStrategy;
    populate_beading_propagation(&mut graph, &strategy);

    let junctions = generate_junctions(&graph, &strategy);

    let rib_back_idx = 0usize;
    let entry = junctions.get(&rib_back_idx);
    assert!(
        entry.is_some(),
        "the rib's upward half (edges[0], boundary -> spine) has NO edge_junctions entry at all \
         -- this is the exact signature of the pre-fix bug: `if !edge.central {{ continue }}` \
         and `if edge.edge_type == EdgeType::EXTRA_VD {{ continue }}` silently dropped every rib \
         from consideration (bug 3 of D-141-JUNCTION-BANDS's correction note), even though \
         canonical generateJunctions has no such gate anywhere and ribs are the primary \
         near-boundary junction carrier."
    );
    let (from_j, to_j) = entry.expect("checked is_some above");
    assert!(
        !from_j.is_empty() && !to_j.is_empty(),
        "the rib's upward half has an edge_junctions entry but it is empty -- expected at least \
         one in-band bead from the spine vertex's bead_count=2 beading"
    );

    let rib_forth_idx = 1usize;
    assert!(
        !junctions.contains_key(&rib_forth_idx),
        "the rib's downward half (edges[1], spine -> boundary) must NOT contribute junctions -- \
         it fails the upward-half-edge selection (from.R > to.R) regardless of the type/\
         centrality gate this test targets"
    );
}
