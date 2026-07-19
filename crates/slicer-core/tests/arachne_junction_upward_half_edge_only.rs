// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
// (`generateJunctions` L2013-2079, `getBeading`/`getNearestBeading` L2091-2127)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Structural test for packet 141 Step 2 (N1) — AC-N1: only the upward
//! half-edge of a twin pair emits junctions.
//!
//! Canonical OrcaSlicer `generateJunctions`
//! (`OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2013-2079`)
//! iterates ALL edges, skipping non-upward half-edges (`from.R > to.R`,
//! L2017) so the OTHER half-edge of the twin pair is responsible for the
//! emission. PNP's prior implementation (`generate_toolpaths.rs:192-334`,
//! finding N1) gated on `edge.central` and processed BOTH half-edges of every
//! central twin pair, producing ~2× the canonical ring length.
//!
//! This test pins the AC-N1 contract: a single central twin-pair edge
//! (R=1 mm -> R=3 mm, bead_count 1 -> 2) yields exactly ONE `edge_junctions`
//! entry, keyed on the upward half-edge (start_vertex = the lower-R vertex).
//! The downward half-edge (start_vertex = the higher-R vertex) MUST NOT
//! appear in the result.
//!
//! Host-only: gated behind `host-algos`, matching the rest of the
//! Arachne junction suite.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::generate_toolpaths::generate_junctions;
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    assign_bead_counts, filter_central, populate_beading_propagation, CentralityParams, EdgeType,
    RibData, STHalfEdge, STVertex, SkeletalTrapezoidationGraph,
};
use slicer_core::voronoi::{Vertex, NO_INDEX};
use slicer_ir::UNITS_PER_MM;

/// A single central twin-pair edge along +x: v0 at the origin (R = 1 mm,
/// bead_count 1) to v1 at 10 mm (R = 3 mm, bead_count 2). Both half-edges
/// are `central = true`, `edge_type = NORMAL` — the canonical twin pair.
///
/// Upward half-edge = `edges[0]` (start_vertex = 0, R goes 1 mm -> 3 mm);
/// its twin `edges[1]` is the downward half (start_vertex = 1, R goes
/// 3 mm -> 1 mm) and MUST be skipped by the canonical `generateJunctions`.
fn single_central_twin_pair() -> SkeletalTrapezoidationGraph {
    let v0 = STVertex {
        position: Vertex { x: 0.0, y: 0.0 },
        distance_to_boundary: 1.0 * UNITS_PER_MM,
        bead_count: Some(1),
        transition_ratio: 0.0,
    };
    let v1 = STVertex {
        position: Vertex {
            x: 10.0 * UNITS_PER_MM,
            y: 0.0,
        },
        distance_to_boundary: 3.0 * UNITS_PER_MM,
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

/// Same `BeadingFactoryParams` as the rest of the propagation/structural
/// suite (see `arachne_beding_propagation_side_table.rs::factory_params`).
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

/// AC-N1 core — a single central twin-pair edge yields ONE entry, keyed on
/// the upward half-edge. The downward half-edge MUST be absent.
///
/// PNP's prior implementation (gated on `central` and processing both
/// halves) would emit a `Some(_)` for BOTH edges[0] AND edges[1]. Canonical
/// `generateJunctions` (L2013-2017) emits only the upward half; this test
/// pins that contract.
#[test]
fn ac_n1_only_upward_half_edge_emits_junctions() {
    let mut graph = single_central_twin_pair();
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    // Populate the side table so the rewrite's `get_beding` lookup is
    // exercised on the canonical path; both vertices have a bead_count so
    // both get a `Some(&Beading)` entry. The assertion below is independent
    // of the beading's exact contents — it pins the *upward-only* emission
    // contract, which is what the rewrite must enforce.
    populate_beading_propagation(&mut graph, strategy.as_ref());

    let junctions = generate_junctions(&graph, strategy.as_ref());

    // Exactly one entry, on the upward half-edge (edges[0], start_vertex = 0).
    assert_eq!(
        junctions.len(),
        1,
        "canonical generateJunctions emits junctions for exactly the upward half of a twin pair \
         (SkeletalTrapezoidation.cpp:2013-2017, skipping from.R > to.R), so a single central \
         twin-pair edge yields exactly one edge_junctions entry. PNP's prior implementation \
         emitted on BOTH halves (generate_toolpaths.rs:192-334, finding N1), producing {} \
         entries here.",
        junctions.len()
    );
    let upward_idx = 0usize;
    assert!(
        junctions.contains_key(&upward_idx),
        "edge_junctions must contain the upward half-edge (edges[0], start_vertex = 0, R goes \
         1 mm -> 3 mm). Canonical generateJunctions picks this half (SkeletalTrapezoidation.cpp:2013-2017)"
    );
    let downward_idx = 1usize;
    assert!(
        !junctions.contains_key(&downward_idx),
        "edge_junctions MUST NOT contain the downward half-edge (edges[1], start_vertex = 1, \
         R goes 3 mm -> 1 mm). Canonical generateJunctions skips from.R > to.R edges \
         (SkeletalTrapezoidation.cpp:2013-2017) — the OTHER half of the twin is responsible \
         for emission. PNP's prior implementation emitted on both halves \
         (generate_toolpaths.rs:198-201, finding N1), so this assertion fails on the un-fixed code"
    );
}

/// AC-N1, geometric signature — the upward half-edge's emitted junctions
/// land on the edge segment between its two vertices, and the
/// `from_junctions` / `to_junctions` are co-located (the canonical
/// algorithm places both at the same parametric position on the edge;
/// the downstream `connectJunctions` chain walk is what assigns them to
/// different vertices in the polyline).
///
/// This pairs with the existence test above to pin the structural AND
/// the geometric shape of the canonical emission in one fixture, without
/// asserting exact coordinates (which depend on the beading strategy's
/// `toolpath_locations` — an implementation detail of
/// `BeadingStrategyFactory` that the structural test does not pin). The
/// AC-1/AC-2 oracle tests in `arachne_parity_red_junction_bands.rs` pin
/// the exact outer-wall placement under the pipeline's default 0.4 mm
/// factory params; this test only pins the canonical DIRECTION (the
/// junction sits on the edge, not at the medial axis or extrapolated
/// past the endpoint).
#[test]
fn ac_n1_upward_half_edge_emits_beads_along_radius_band() {
    let mut graph = single_central_twin_pair();
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    populate_beading_propagation(&mut graph, strategy.as_ref());

    let junctions = generate_junctions(&graph, strategy.as_ref());
    let upward_idx = 0usize;
    let junctions_vec = junctions.get(&upward_idx).unwrap_or_else(|| {
        panic!(
            "upward half-edge (edges[0]) must have an entry in edge_junctions (AC-N1 core); \
             got keys = {:?}",
            junctions.keys().collect::<Vec<_>>()
        )
    });

    assert!(
        !junctions_vec.is_empty(),
        "junctions on the upward half-edge must be non-empty (in-band beads exist)"
    );

    // Every emitted junction sits on the edge segment between the two
    // endpoints (x ∈ [0, 10] mm for this fixture). PNP's prior
    // clamp-based emission extrapolated past the endpoint on out-of-band
    // beads; the canonical algorithm's in-band-only emission
    // (`SkeletalTrapezoidation.cpp:2064-2077`) places every junction
    // strictly on the edge.
    for j in junctions_vec.iter() {
        assert!(
            j.p.x >= -0.01 && j.p.x <= 10.01,
            "junction at ({:.3}, {:.3}) mm lies OFF the edge segment x ∈ [0, 10] mm — canonical \
             generateJunctions places every junction on the edge (in-band emission, no \
             extrapolation; SkeletalTrapezoidation.cpp:2064-2077). PNP's prior implementation \
             extrapolated past the endpoint via `.clamp(0.0, 1.0)` (generate_toolpaths.rs:309-318, \
             finding N1)",
            j.p.x,
            j.p.y
        );
    }
    // The flat `LineJunctions`-style storage holds one junction per bead per
    // edge (canonical computes ONE parametric position per bead and the
    // downstream `chain_junctions_for_bead` walk assigns it to a vertex), so
    // there is no separate `from`/`to` co-location check to perform here.
}

// ---------------------------------------------------------------------------
// Wiring sanity check — the structural test only uses `filter_central` +
// `assign_bead_counts` + `populate_beading_propagation` + `generate_junctions`
// in isolation, so re-assert that those APIs are reachable from this crate's
// integration-test surface. Catches future refactors that would break the
// test's wiring silently.
// ---------------------------------------------------------------------------

#[test]
fn ac_n1_wiring_integration_steps_reachable() {
    // Build the fixture the same way as the two AC-N1 tests above — if
    // either call site's signature drifts, this test fails first and the
    // structural tests give a more focused error.
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(&[{
        use slicer_ir::{ExPolygon, Point2, Polygon};
        let s = (20.0 * UNITS_PER_MM) as i64;
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 { x: s, y: 0 },
                    Point2 { x: s, y: s },
                    Point2 { x: 0, y: s },
                ],
            },
            holes: Vec::new(),
        }
    }])
    .expect("20mm square should build");
    let params = CentralityParams::new(0.01 * UNITS_PER_MM, 0.0);
    filter_central(&mut graph, &params, std::f64::consts::PI);
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    assign_bead_counts(&mut graph, strategy.as_ref()).expect("filter_central just ran");
    populate_beading_propagation(&mut graph, strategy.as_ref());
    let _ = generate_junctions(&graph, strategy.as_ref());
}
