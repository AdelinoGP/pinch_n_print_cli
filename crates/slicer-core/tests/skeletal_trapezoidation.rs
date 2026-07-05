//! Step 1 tests for the synthetic quad/rib topology pass.
//!
//! Host-only: `skeletal_trapezoidation` is gated behind the `host-algos`
//! feature, so this whole file is a no-op under default features.

#![cfg(feature = "host-algos")]

use slicer_core::skeletal_trapezoidation::{
    build_quad_rib_topology, EdgeType, SkeletalTrapezoidationGraph,
};
use slicer_ir::{ExPolygon, Point2, Polygon};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// AC-1 + AC-N1: a square has no sharp corners that generate ribs, so
/// `build_quad_rib_topology` produces zero quad cells and every edge stays
/// `EdgeType::NORMAL`.
#[test]
fn quad_rib_topology_square_has_no_ribs() {
    let square = expoly(vec![p(0, 0), p(1000, 0), p(1000, 1000), p(0, 1000)]);
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(&square))
        .expect("square should build a skeletal graph");

    build_quad_rib_topology(&mut graph).expect("rib topology should build");

    assert!(
        graph.rib.quad_cells.is_empty(),
        "square should have no quad cells (no sharp corners), got {}",
        graph.rib.quad_cells.len()
    );

    for (i, edge) in graph.edges.iter().enumerate() {
        assert_eq!(
            edge.edge_type,
            EdgeType::NORMAL,
            "edge {i} on a square should be NORMAL; got {:?}",
            edge.edge_type
        );
    }
}

/// AC-N4: two independent runs over the same input produce identical quad cells
/// and edge-type classifications.
#[test]
fn quad_rib_topology_is_deterministic() {
    let wedge = expoly(vec![p(0, 0), p(10_000, -100), p(10_000, 100)]);

    let mut g1 = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(&wedge))
        .expect("wedge should build a skeletal graph");
    build_quad_rib_topology(&mut g1).expect("rib topology should build");

    let mut g2 = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(&wedge))
        .expect("wedge should build a skeletal graph");
    build_quad_rib_topology(&mut g2).expect("rib topology should build");

    assert_eq!(
        g1.rib.quad_cells, g2.rib.quad_cells,
        "quad cells should be deterministic across two runs"
    );

    assert_eq!(
        g1.edges.len(),
        g2.edges.len(),
        "edge counts should match across two runs"
    );
    for (i, (e1, e2)) in g1.edges.iter().zip(g2.edges.iter()).enumerate() {
        assert_eq!(
            e1.edge_type, e2.edge_type,
            "edge-type classification diverged at edge {i}"
        );
        assert_eq!(
            e1.rib_twin, e2.rib_twin,
            "rib_twin assignment diverged at edge {i}"
        );
        assert_eq!(
            e1.quad_cell, e2.quad_cell,
            "quad_cell assignment diverged at edge {i}"
        );
    }
}
