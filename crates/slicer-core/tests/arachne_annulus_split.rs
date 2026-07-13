//! Reproduction harness for the long-standing "missing/merged outer walls" bug.
//!
//! An annulus (region + hole) exercises the same skeletal-topology case the
//! benchy hull hits at Z=9.6/17.2/20.6: the medial axis connects the OUTER
//! contour to the INNER hole contour through 3-way spine vertices. A faithful
//! Arachne `connectJunctions`+`addToolpathSegment` must emit the outer wall
//! and the hole's outer wall as SEPARATE `ExtrusionLine`s (split at the 3-way
//! odd vertices); the current flatten-then-emit-one-line-per-bead walk
//! concatenates them into one wrong open path, dropping the real hull outline.
//!
//! This is NOT a self-captured baseline: it asserts the geometrically correct
//! property (separate closed loops for outer contour and hole), which is what
//! OrcaSlicer produces and what the benchy needs.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::generate_toolpaths;
use slicer_core::arachne::stitch::stitch_extrusions;
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    apply_transitions, assign_bead_counts, filter_central, generate_transition_mids,
    propagate_beadings_downward, propagate_beadings_upward, CentralityParams,
    SkeletalTrapezoidationGraph,
};
use slicer_ir::{ExPolygon, ExtrusionLine, Point2, Polygon, UNITS_PER_MM};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn factory_params() -> BeadingFactoryParams {
    BeadingFactoryParams {
        optimal_width: 20.0,
        default_transition_length: 20.0,
        transition_filter_dist: 10.0,
        distribution_count: 1,
        min_input_width: 5.0,
        min_output_width: 20.0,
        outer_wall_offset: 0.0,
        max_bead_count: 9,
        minimum_variable_line_ratio: 0.5,
        print_thin_walls: false,
        preferred_bead_width_outer: 20.0,
        wall_transition_angle: 0.17453292519943295,
        initial_layer_min_bead_width: 20.0,
    }
}

fn centrality_params() -> CentralityParams {
    CentralityParams::new(200.0, 50.0)
}
fn run_pipeline(poly: &ExPolygon) -> Vec<ExtrusionLine> {
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph");
    let mut centrality_params = centrality_params();
    centrality_params.transition_filter_dist *= 0.01;
    filter_central(&mut graph, &centrality_params, std::f64::consts::PI);
    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    assign_bead_counts(&mut graph, strategy.as_ref()).expect("assign");
    generate_transition_mids(&mut graph, strategy.as_ref());
    apply_transitions(&mut graph);
    propagate_beadings_upward(&mut graph);
    propagate_beadings_downward(&mut graph);
    let lines = generate_toolpaths(&graph, strategy.as_ref());
    // Flatten buckets (generate_toolpaths returns one bucket per bead index),
    // then run the SAME stitch pass the production pipeline uses so per-quad
    // rungs rejoin into closed loops and spine-separated contours stay apart.
    let flat: Vec<ExtrusionLine> = lines.into_iter().flatten().collect();
    stitch_extrusions(flat, 0.4 * UNITS_PER_MM)
}

/// Outer square (2mm) with a centred square hole (1mm) — same topology as the
/// benchy hull + hollow-interior at the broken layers.
fn annulus() -> ExPolygon {
    let outer = Polygon {
        points: vec![
            p(-10_000, -10_000),
            p(10_000, -10_000),
            p(10_000, 10_000),
            p(-10_000, 10_000),
        ],
    };
    let hole = Polygon {
        points: vec![
            p(-5_000, -5_000),
            p(5_000, -5_000),
            p(5_000, 5_000),
            p(-5_000, 5_000),
        ],
    };
    ExPolygon {
        contour: outer,
        holes: vec![hole],
    }
}

#[test]
fn annulus_outer_and_hole_are_separate_closed_loops() {
    let out = run_pipeline(&annulus());
    // In a region with a hole, inset 0 is the OUTER contour only; the hole's
    // wall is a higher inset (the innermost bead of the same region). The two
    // must remain SEPARATE closed loops and must never be concatenated into one
    // open path (the benchy hull "missing walls" bug). With the bug, the outer
    // hull loop is replaced by a single merged open path spanning both
    // contours.
    let mut by_inset: std::collections::BTreeMap<u32, Vec<&ExtrusionLine>> =
        std::collections::BTreeMap::new();
    for l in &out {
        by_inset.entry(l.inset_idx).or_default().push(l);
    }
    let total_closed: usize = by_inset
        .values()
        .map(|v| v.iter().filter(|l| l.is_closed).count())
        .sum();
    for (k, v) in &by_inset {
        println!(
            "inset{}: lines={} closed={} sizes={:?}",
            k,
            v.len(),
            v.iter().filter(|l| l.is_closed).count(),
            v.iter().map(|l| l.junctions.len()).collect::<Vec<_>>()
        );
    }
    // Outer contour must be exactly one closed loop in inset 0.
    let outer = by_inset.get(&0).expect("inset 0 must exist");
    let outer_closed = outer.iter().filter(|l| l.is_closed).count();
    assert_eq!(
        outer_closed, 1,
        "inset 0 must be exactly one closed outer loop; got {outer_closed}"
    );
    // The hole must be a separate closed loop at a higher inset (not merged
    // into the outer loop, and not dropped).
    assert!(
        total_closed >= 2,
        "outer and hole must both be present as separate closed loops; got {total_closed}"
    );
    // No merged fragment in inset 0: every line there is the closed outer loop.
    assert!(
        outer.len() == outer_closed,
        "inset 0 must contain only the outer loop, no merged fragment"
    );
}
