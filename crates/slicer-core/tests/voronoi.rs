#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

//! Packet 113c (AC-1) acceptance fixture for `slicer_core::voronoi`.
//!
//! Covers `HalfEdgeGraph::cells`/`VCell`: per-cell Voronoi metadata mirroring
//! `boostvoronoi::Cell`, needed by later packet-113c steps' per-cell
//! `transferEdge`/`makeRib` walk.

use slicer_core::voronoi::{voronoi_from_segments, Segment, NO_INDEX};
use slicer_ir::Point2;

fn seg(a: Point2, b: Point2) -> Segment {
    Segment { a, b }
}

/// AC-1: a plain 10mm square polygon's four segments produce a non-empty
/// `HalfEdgeGraph::cells`, and every `VCell`'s fields are accessible and
/// sane: `incident_edge` resolves to a valid edge index (or `NO_INDEX` only
/// when `is_degenerate`), and `source_index` is in range of the input
/// segment count.
#[test]
fn voronoi_cells_square_metadata() {
    let segments = [
        seg(Point2::from_mm(0.0, 0.0), Point2::from_mm(10.0, 0.0)),
        seg(Point2::from_mm(10.0, 0.0), Point2::from_mm(10.0, 10.0)),
        seg(Point2::from_mm(10.0, 10.0), Point2::from_mm(0.0, 10.0)),
        seg(Point2::from_mm(0.0, 10.0), Point2::from_mm(0.0, 0.0)),
    ];

    let graph = match voronoi_from_segments(&segments) {
        Ok(graph) => graph,
        Err(err) => panic!("square fixture should build, got {err}"),
    };

    assert!(
        !graph.cells.is_empty(),
        "expected a non-empty cell list for a 4-segment square"
    );

    for cell in &graph.cells {
        // Exactly one geometry category per cell.
        assert!(
            cell.contains_point ^ cell.contains_segment,
            "cell must contain either a point site or a segment site, not both/neither: {cell:?}"
        );

        if cell.contains_segment {
            // A segment-site cell must not also claim to be a
            // segment-endpoint-site cell.
            assert!(!cell.contains_segment_startpoint);
            assert!(!cell.contains_segment_endpoint);
        }

        // source_index must index into the original 4-segment input.
        assert!(
            cell.source_index < segments.len(),
            "source_index {} out of range for {} input segments",
            cell.source_index,
            segments.len()
        );

        if cell.is_degenerate {
            assert_eq!(
                cell.incident_edge, NO_INDEX,
                "degenerate cell must report NO_INDEX for incident_edge"
            );
        } else {
            assert_ne!(
                cell.incident_edge, NO_INDEX,
                "non-degenerate cell must have a real incident_edge"
            );
            assert!(
                cell.incident_edge < graph.edges.len(),
                "incident_edge {} out of range for {} edges",
                cell.incident_edge,
                graph.edges.len()
            );
        }
    }
}
