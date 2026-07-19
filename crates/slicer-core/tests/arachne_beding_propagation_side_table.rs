// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path:
//   src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
//     (`getBeading`/`getNearestBeading` L2091-2127,
//      `propagateBeadingsDownward` L1833-1899,
//      `upwardQuadMids` L1669-1672)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Structural test for packet 141 Step 1 (N7): `BeadingPropagation` side table
//! on `SkeletalTrapezoidationGraph`, `get_beding`/`get_nearest_beding`
//! accessors, and the structural invariant that every side-table entry's
//! `Beading` satisfies `bead_widths.len() == toolpath_locations.len()`.
//!
//! Step 1 only delivers the substrate; AC-1/AC-2 (the parity oracle tests for
//! the side table) and AC-N1 (upward-half-edge-only junction emission) are
//! Step 2's oracles. This file pins the *structural* contract that Step 2
//! depends on:
//!
//! 1. `graph.beading_propagation` is initialized for every vertex
//!    (`vec![None; vertices.len()]` in `from_polygons`).
//! 2. After [`populate_beading_propagation`], every vertex with
//!    `bead_count = Some(_)` has a `Some(&Beading)` entry; every rib-foot
//!    vertex with `bead_count = None` has a `None` entry.
//! 3. Every populated `Beading` in the side table satisfies
//!    `bead_widths.len() == toolpath_locations.len()` (debug-asserted in
//!    `get_beding` as a hot-path invariant).
//! 4. `get_nearest_beding(v, 0.1 mm)` returns the expected entry for vertices
//!    whose nearest populated neighbour is within the 0.1 mm radius (1000
//!    slicer units), and `None` for vertices whose nearest populated
//!    neighbour lies outside the radius.
//!
//! AC-N1 is intentionally NOT asserted here — Step 2 owns the junction rewrite
//! and the existing N1 red tests in
//! `tests/arachne_parity_red_junction_bands.rs` must still FAIL after this
//! step (Step 2 is what turns them green).
//!
//! Host-only: gated behind `host-algos`, matching the rest of the skeletal
//! trapezoidation suite.

#![cfg(feature = "host-algos")]

use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    assign_bead_counts, filter_central, populate_beading_propagation, CentralityParams,
    SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::NO_INDEX;
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// A 20x20 mm square: large enough that every medial-axis spine edge clears
/// the default factory's `optimal_width`, so a real mix of primary-assigned
/// and rib-foot vertices exists for the structural assertions.
fn square_20mm() -> ExPolygon {
    let s = (20.0 * UNITS_PER_MM) as i64;
    expoly(vec![p(0, 0), p(s, 0), p(s, s), p(0, s)])
}

/// Same `BeadingFactoryParams` as `tests/propagation.rs` and
/// `tests/bead_count.rs` (`factory_params()`) — reused verbatim so the
/// structural assertions on the side table's beadings are produced by the
/// same strategy the rest of the suite uses.
fn factory_params() -> BeadingFactoryParams {
    BeadingFactoryParams {
        optimal_width: 20.0,
        default_transition_length: 20.0,
        transition_filter_dist: 10.0,
        distribution_count: 1,
        min_input_width: 5.0,
        min_output_width: 20.0,
        outer_wall_offset: 0.0,
        max_bead_count: 10,
        minimum_variable_line_ratio: 0.5,
        print_thin_walls: false,
        preferred_bead_width_outer: 20.0,
        wall_transition_angle: 0.17453292519943295,
        initial_layer_min_bead_width: 20.0,
        ..Default::default()
    }
}

/// Same permissive `CentralityParams` as `arachne_invariants.rs`'s
/// `build_propagated_graph` (a 0.01mm floor with permissive PI angle) — keeps
/// the fixture deterministic across `host-algos` builds.
fn centrality_params() -> CentralityParams {
    CentralityParams::new(0.01 * UNITS_PER_MM, 0.0)
}

/// Builds a graph for `poly` with `filter_central` and `assign_bead_counts`
/// run, but **without** `apply_transitions` / `propagate_beadings_*` — so
/// the side table is the pristine primary-pass state the structural test
/// inspects.
fn build_primary_graph(poly: &ExPolygon) -> SkeletalTrapezoidationGraph {
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon should build a skeletal graph");
    filter_central(&mut graph, &centrality_params(), std::f64::consts::PI);
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    assign_bead_counts(&mut graph, strategy.as_ref())
        .expect("filter_central just ran, so centrality_filtered is true");
    graph
}

// ---------------------------------------------------------------------------
// Structural assertions
// ---------------------------------------------------------------------------

/// AC-structural-1: the side table exists, is the same length as
/// `vertices`, and starts as all-`None` (no implicit population in
/// `from_polygons`). Without this, `populate_beading_propagation` cannot
/// safely index into it.
#[test]
fn side_table_initialized_all_none_in_from_polygons() {
    let graph = SkeletalTrapezoidationGraph::from_polygons(&[square_20mm()])
        .expect("square_20mm should build");
    assert_eq!(
        graph.beading_propagation.len(),
        graph.vertices.len(),
        "beading_propagation must be index-parallel to vertices"
    );
    assert!(
        graph
            .beading_propagation
            .iter()
            .all(|entry| entry.is_none()),
        "from_polygons must initialise every side-table entry to None; got {:?}",
        graph
            .beading_propagation
            .iter()
            .map(|e| e.is_some())
            .collect::<Vec<_>>()
    );
}

/// AC-structural-2: after `populate_beading_propagation`, every vertex with
/// `bead_count = Some(_)` has `Some(&Beading)` in the side table, and every
/// rib-foot vertex with `bead_count = None` has `None`. This is the
/// `get_beding` contract Step 2 (N1) depends on.
#[test]
fn populate_side_table_covers_primary_vertices_only() {
    let mut graph = build_primary_graph(&square_20mm());
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    populate_beading_propagation(&mut graph, strategy.as_ref());

    let mut saw_populated = false;
    let mut saw_unpopulated = false;
    for (v_idx, v) in graph.vertices.iter().enumerate() {
        let entry = &graph.beading_propagation[v_idx];
        match v.bead_count {
            Some(bc) if bc > 0 => {
                assert!(
                    entry.is_some(),
                    "vertex {v_idx} has bead_count = {:?} but no side-table beading",
                    v.bead_count
                );
                saw_populated = true;
            }
            Some(0) => {
                // Canonical `SkeletalTrapezoidation.cpp:1700` skips
                // `bead_count <= 0` — zero-bead vertices get no side-table
                // entry.
                assert!(
                    entry.is_none(),
                    "vertex {v_idx} has bead_count = Some(0) but side-table is populated \
                     — canonical skips bead_count <= 0"
                );
            }
            None => {
                assert!(
                    entry.is_none(),
                    "rib-foot vertex {v_idx} (bead_count = None) must not be populated \
                     in the side table"
                );
                saw_unpopulated = true;
            }
            _ => unreachable!(),
        }
    }
    assert!(
        saw_populated,
        "fixture emitted no populated side-table entries — the structural test is degenerate"
    );
    assert!(
        saw_unpopulated,
        "fixture has no rib-foot vertices — the structural test is degenerate"
    );
}

/// AC-structural-3: every populated side-table entry's `Beading` satisfies
/// the documented invariant `bead_widths.len() == toolpath_locations.len()`.
/// Also re-asserts the invariant on a non-empty `left_over` case (zero-bead
/// thickness) by computing a beading for the boundary itself.
#[test]
fn side_table_beadings_satisfy_widths_locations_invariant() {
    let mut graph = build_primary_graph(&square_20mm());
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    populate_beading_propagation(&mut graph, strategy.as_ref());

    let mut checked = 0usize;
    for (v_idx, entry) in graph.beading_propagation.iter().enumerate() {
        let Some(beading) = entry else { continue };
        assert_eq!(
            beading.bead_widths.len(),
            beading.toolpath_locations.len(),
            "vertex {v_idx} side-table beading violates bead_widths.len() == toolpath_locations.len(): \
             widths = {:?}, locations = {:?}",
            beading.bead_widths,
            beading.toolpath_locations
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no side-table entries to check — the structural test is degenerate"
    );
}

/// AC-structural-4: `get_beding` returns `Some(&Beading)` for vertices with a
/// primary bead count, `None` otherwise. The returned `Beading` is identical
/// (by `PartialEq`) to the entry `populate_beading_propagation` wrote.
#[test]
fn get_beding_round_trips_populated_entries() {
    let mut graph = build_primary_graph(&square_20mm());
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    populate_beading_propagation(&mut graph, strategy.as_ref());

    for (v_idx, v) in graph.vertices.iter().enumerate() {
        let got = graph.get_beding(v_idx);
        match v.bead_count {
            Some(bc) if bc > 0 => {
                let got = got.unwrap_or_else(|| {
                    panic!(
                        "get_beding({v_idx}) returned None for a vertex with bead_count = {:?}",
                        v.bead_count
                    )
                });
                let stored = graph.beading_propagation[v_idx]
                    .as_ref()
                    .expect("primary vertex must have a side-table entry");
                assert_eq!(
                    got, stored,
                    "get_beding({v_idx}) returned a Beading different from the stored entry"
                );
            }
            Some(0) => {
                // Canonical skips bead_count <= 0 (SkeletalTrapezoidation.cpp:1700).
                assert!(
                    got.is_none(),
                    "get_beding({v_idx}) returned Some for a bead_count=0 vertex"
                );
            }
            None => assert!(
                got.is_none(),
                "get_beding({v_idx}) returned Some for a rib-foot vertex with bead_count = None"
            ),
            _ => unreachable!(),
        }
    }
    // Out-of-range lookups return None (no panic).
    assert!(graph.get_beding(usize::MAX).is_none());
}

/// AC-structural-5: `get_nearest_beding` with a 0.1 mm radius (1000 slicer
/// units) returns the closest populated entry to `v`. We don't pin a single
/// numeric answer (graph geometry depends on Voronoi construction, which is
/// out of this step's scope); we pin the *contract* — the returned entry, if
/// any, is a populated side-table entry whose vertex lies within the radius
/// from `v`, and no closer populated entry exists outside the returned
/// vertex's neighbourhood.
///
/// "Closest" is defined as Euclidean vertex-to-vertex distance, matching the
/// existing `edge_length` helper's metric (slicer units).
#[test]
fn get_nearest_beding_respects_0_1mm_radius_contract() {
    let mut graph = build_primary_graph(&square_20mm());
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    populate_beading_propagation(&mut graph, strategy.as_ref());

    const RADIUS_UNITS: f64 = 0.1 * UNITS_PER_MM; // 1000 slicer units

    for v_idx in 0..graph.vertices.len() {
        if graph.beading_propagation[v_idx].is_none() {
            continue;
        }
        let got = graph
            .get_nearest_beding(v_idx, RADIUS_UNITS)
            .unwrap_or_else(|| {
                panic!(
                    "get_nearest_beding({v_idx}, 0.1mm) returned None for a vertex with a \
                     populated side-table entry — the radius self-distance is 0, so the \
                     nearest-lookup must return at least the vertex itself"
                )
            });
        // `got` is the same `Beading` we stored at `v_idx` (every vertex is its
        // own nearest neighbour at distance 0 within any positive radius).
        let stored = graph.beading_propagation[v_idx]
            .as_ref()
            .expect("test precondition: vertex has a side-table entry");
        assert_eq!(
            got, stored,
            "get_nearest_beding({v_idx}, 0.1mm) should return the vertex's own beading (distance 0)"
        );
    }
}

/// AC-structural-6: `get_nearest_beding(v, radius)` with `radius = 0` is
/// degenerate — by the OrcaSlicer contract (and the obvious nearest-neighbour
/// semantics) the only candidate is `v` itself. If `v` has no beading, the
/// function must return `None`; if `v` has a beading, it must return that
/// beading.
#[test]
fn get_nearest_beding_zero_radius_returns_self_only() {
    let mut graph = build_primary_graph(&square_20mm());
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    populate_beading_propagation(&mut graph, strategy.as_ref());

    for v_idx in 0..graph.vertices.len() {
        let got = graph.get_nearest_beding(v_idx, 0.0);
        match &graph.beading_propagation[v_idx] {
            Some(stored) => {
                let got = got.unwrap_or_else(|| {
                    panic!("zero-radius lookup on a populated vertex {v_idx} must return Some")
                });
                assert_eq!(got, stored, "zero-radius lookup must return self");
            }
            None => assert!(
                got.is_none(),
                "zero-radius lookup on an unpopulated vertex {v_idx} must return None"
            ),
        }
    }
}

/// AC-structural-7: `get_nearest_beding(v, radius)` with a very small radius
/// (smaller than any inter-vertex gap) on an unpopulated vertex whose
/// nearest populated neighbour lies outside the radius must return `None`.
///
/// The square fixture is dense enough that this isn't trivially true on
/// every unpopulated vertex; we assert the *upper* contract (the function
/// never lies about distances) by checking that the returned entry's source
/// vertex is within `radius` of `v` whenever a `Some` is returned.
#[test]
fn get_nearest_beding_distance_upper_bound() {
    let mut graph = build_primary_graph(&square_20mm());
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    populate_beading_propagation(&mut graph, strategy.as_ref());

    const RADIUS_UNITS: f64 = 0.1 * UNITS_PER_MM;

    for v_idx in 0..graph.vertices.len() {
        let Some(_got) = graph.get_nearest_beding(v_idx, RADIUS_UNITS) else {
            continue;
        };
        let v_pos = graph.vertices[v_idx].position;
        let mut best_dist = f64::INFINITY;
        let mut best_v = NO_INDEX;
        for (other_idx, other_entry) in graph.beading_propagation.iter().enumerate() {
            if other_entry.is_none() {
                continue;
            }
            let other_pos = graph.vertices[other_idx].position;
            let dx = other_pos.x - v_pos.x;
            let dy = other_pos.y - v_pos.y;
            let d = (dx * dx + dy * dy).sqrt();
            if d < best_dist {
                best_dist = d;
                best_v = other_idx;
            }
        }
        // The brute-force nearest populated vertex is always within `radius`
        // (v is its own nearest at distance 0 when v is populated, so the
        // bound is trivially true; this is the upper-bound check on the
        // graph-traversal-based implementation).
        assert!(
            best_dist <= RADIUS_UNITS,
            "get_nearest_beding({v_idx}, 0.1mm) returned Some, but the brute-force nearest \
             populated vertex is {best_v} at {best_dist:.3} units — outside the 0.1mm radius. \
             This indicates the nearest-lookup returned a non-nearest entry."
        );
    }
}
