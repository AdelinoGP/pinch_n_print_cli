//! Red tests encoding finding **N10** of the second-pass Arachne parity audit
//! (`target/arachne_parity_audit_20260706_020657.md`, §N10).
//!
//! **Finding N10:** PNP's `SkeletalTrapezoidationGraph::from_polygons`
//! (`graph.rs:306-371`) lacked the canonical construction epilogue
//! (`SkeletalTrapezoidation.cpp:538-546`): `separatePointyQuadEndNodes`
//! (duplicate shared boundary start-nodes), `collapseSmallEdges` (remove
//! degenerate zero-length edges), and incident-edge normalization (documented
//! no-op in PNP). Without the epilogue, zero-length spine fragments survive
//! into centrality/junction math, and pointy-corner cells share quad-start
//! nodes.
//!
//! Host-only: gated behind `host-algos`.

#![cfg(feature = "host-algos")]

use slicer_core::skeletal_trapezoidation::SkeletalTrapezoidationGraph;
use slicer_core::voronoi::NO_INDEX;
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

fn p_mm(x_mm: f64, y_mm: f64) -> Point2 {
    Point2 {
        x: (x_mm * UNITS_PER_MM) as i64,
        y: (y_mm * UNITS_PER_MM) as i64,
    }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// AC-2, part 1 — no zero-length edges after `collapseSmallEdges`.
///
/// A polygon with integer-coordinate vertices (in mm) whose Voronoi diagram
/// may produce degenerate zero-length edges from integer rounding. After
/// `from_polygons` runs (with the N10 epilogue), no edge should have both
/// endpoints at the same position (within the 5 nm snap distance).
#[test]
fn ac2_no_zero_length_edges() {
    // A triangle with integer mm coordinates — likely to produce
    // degenerate VD edges from rounding.
    let tri = expoly(vec![p_mm(0.0, 0.0), p_mm(20.0, 0.0), p_mm(10.0, 17.0)]);

    let graph = SkeletalTrapezoidationGraph::from_polygons(&[tri])
        .expect("triangle should build successfully");

    // 5 nm snap distance squared, in PNP units.
    let snap_dist_sq = (0.05_f64).powi(2);

    let mut degenerate_count = 0usize;
    for edge in graph.edges.iter() {
        if edge.start_vertex == NO_INDEX {
            continue; // Collapsed edge.
        }
        let twin = edge.twin;
        if twin == NO_INDEX {
            continue;
        }
        let from_v = edge.start_vertex;
        let to_v = graph.edges[twin].start_vertex;
        if from_v == NO_INDEX || to_v == NO_INDEX {
            continue;
        }
        let from = &graph.vertices[from_v].position;
        let to = &graph.vertices[to_v].position;
        let dx = from.x - to.x;
        let dy = from.y - to.y;
        if dx * dx + dy * dy < snap_dist_sq {
            degenerate_count += 1;
        }
    }

    assert_eq!(
        degenerate_count, 0,
        "expected no zero-length edges after collapseSmallEdges epilogue, found {}",
        degenerate_count
    );
}

/// AC-2, part 2 — unique quad-start nodes after `separatePointyQuadEndNodes`.
///
/// A polygon with pointy corners whose Voronoi diagram may produce quads
/// sharing boundary start-nodes. After `from_polygons` runs (with the N10
/// epilogue), every chain head (`prev == NO_INDEX`) should have a unique
/// `start_vertex`.
#[test]
fn ac2_unique_quad_start_nodes() {
    // A triangle — pointy corners produce shared start-nodes.
    let tri = expoly(vec![p_mm(0.0, 0.0), p_mm(20.0, 0.0), p_mm(10.0, 17.0)]);

    let graph = SkeletalTrapezoidationGraph::from_polygons(&[tri])
        .expect("triangle should build successfully");

    let mut seen_starts = std::collections::HashSet::new();
    let mut duplicate_count = 0usize;

    for edge in graph.edges.iter() {
        if edge.prev != NO_INDEX {
            continue; // Not a chain head.
        }
        if edge.start_vertex == NO_INDEX {
            continue; // Collapsed edge.
        }
        if !seen_starts.insert(edge.start_vertex) {
            duplicate_count += 1;
        }
    }

    assert_eq!(
        duplicate_count, 0,
        "expected no duplicate quad-start nodes after separatePointyQuadEndNodes epilogue, \
         found {} duplicates",
        duplicate_count
    );
}
